mod ffi;

use engine::{render_document, DisplayCommand};
use engine_loop::Scheduler;
#[cfg(feature = "process-split")]
use ipc::{BrowserToContent, InProcessTransport};
use platform_abi::{
    PlatformConfig, PlatformEvent, PlatformFrame, PLATFORM_ABI_VERSION, PLATFORM_EVENT_KEY_DOWN,
    PLATFORM_EVENT_QUIT, PLATFORM_EVENT_RESIZE, PLATFORM_FALSE, PLATFORM_KEY_ESCAPE,
};
use renderer::{DrawRect, OverlayInfo, Pattern, Renderer};
use script_host::{ScriptError, ScriptHost, StubScriptHost};
use std::{
    ffi::CString,
    fs,
    mem::MaybeUninit,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone)]
enum Command {
    Run(RunArgs),
    Headless(HeadlessArgs),
    Golden(GoldenArgs),
}

#[derive(Debug, Clone)]
struct RunArgs {
    pattern: Pattern,
    input: Option<PathBuf>,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
struct HeadlessArgs {
    input: PathBuf,
    width: u32,
    height: u32,
    frame: u64,
    out: PathBuf,
}

#[derive(Debug, Clone)]
struct GoldenArgs {
    update: bool,
    fixture_dir: PathBuf,
    golden_dir: PathBuf,
    width: u32,
    height: u32,
    frame: u64,
}

#[derive(Debug, Clone)]
struct DocumentScene {
    html: String,
    rects: Vec<DrawRect>,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("browser failed: {err}");
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), String> {
    let command = parse_cli(std::env::args().skip(1))?;
    process_split_bootstrap();

    match command {
        Command::Run(args) => run_windowed(args),
        Command::Headless(args) => run_headless(args),
        Command::Golden(args) => run_golden(args),
    }
}

fn parse_cli(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut args: Vec<String> = args.collect();

    if args.is_empty() {
        return Ok(Command::Run(RunArgs {
            pattern: Pattern::Gradient,
            input: None,
            width: 960,
            height: 540,
        }));
    }

    let command = args.remove(0);
    match command.as_str() {
        "run" => parse_run_args(args.into_iter()),
        "headless" => parse_headless_args(args.into_iter()),
        "golden" => parse_golden_args(args.into_iter()),
        flag if flag.starts_with("--") => {
            parse_run_args(std::iter::once(flag.to_string()).chain(args))
        }
        other => Err(format!(
            "unknown command '{other}' (expected: run|headless|golden)"
        )),
    }
}

fn parse_run_args(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut pattern = Pattern::Gradient;
    let mut input = None;
    let mut width = 960_u32;
    let mut height = 540_u32;

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--pattern" => {
                let value = next_arg(&mut args, "--pattern")?;
                pattern = Pattern::parse(&value).ok_or_else(|| {
                    format!("unknown pattern '{value}' (expected: gradient|solid|rects)")
                })?;
            }
            "--input" => {
                input = Some(PathBuf::from(next_arg(&mut args, "--input")?));
            }
            "--width" => {
                width = parse_u32(&next_arg(&mut args, "--width")?, "--width")?;
            }
            "--height" => {
                height = parse_u32(&next_arg(&mut args, "--height")?, "--height")?;
            }
            _ => return Err(format!("unknown run flag '{arg}'")),
        }
    }

    Ok(Command::Run(RunArgs {
        pattern,
        input,
        width,
        height,
    }))
}

fn parse_headless_args(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut input = None;
    let mut out = None;
    let mut width = 960_u32;
    let mut height = 540_u32;
    let mut frame = 0_u64;

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                input = Some(PathBuf::from(next_arg(&mut args, "--input")?));
            }
            "--out" => {
                out = Some(PathBuf::from(next_arg(&mut args, "--out")?));
            }
            "--width" => {
                width = parse_u32(&next_arg(&mut args, "--width")?, "--width")?;
            }
            "--height" => {
                height = parse_u32(&next_arg(&mut args, "--height")?, "--height")?;
            }
            "--frame" => {
                frame = parse_u64(&next_arg(&mut args, "--frame")?, "--frame")?;
            }
            _ => return Err(format!("unknown headless flag '{arg}'")),
        }
    }

    let input = input.ok_or_else(|| "headless requires --input <path>".to_string())?;
    let out = out.ok_or_else(|| "headless requires --out <path>".to_string())?;

    Ok(Command::Headless(HeadlessArgs {
        input,
        width,
        height,
        frame,
        out,
    }))
}

fn parse_golden_args(args: impl Iterator<Item = String>) -> Result<Command, String> {
    let mut update = false;
    let mut fixture_dir = PathBuf::from("tests/fixtures");
    let mut golden_dir = PathBuf::from("tests/golden");
    let mut width = 960_u32;
    let mut height = 540_u32;
    let mut frame = 0_u64;

    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--update" => update = true,
            "--fixture-dir" => {
                fixture_dir = PathBuf::from(next_arg(&mut args, "--fixture-dir")?);
            }
            "--golden-dir" => {
                golden_dir = PathBuf::from(next_arg(&mut args, "--golden-dir")?);
            }
            "--width" => {
                width = parse_u32(&next_arg(&mut args, "--width")?, "--width")?;
            }
            "--height" => {
                height = parse_u32(&next_arg(&mut args, "--height")?, "--height")?;
            }
            "--frame" => {
                frame = parse_u64(&next_arg(&mut args, "--frame")?, "--frame")?;
            }
            _ => return Err(format!("unknown golden flag '{arg}'")),
        }
    }

    Ok(Command::Golden(GoldenArgs {
        update,
        fixture_dir,
        golden_dir,
        width,
        height,
        frame,
    }))
}

fn run_windowed(args: RunArgs) -> Result<(), String> {
    let title = CString::new("Browser")
        .map_err(|_| "window title contains interior null byte".to_string())?;

    let mut width = args.width;
    let mut height = args.height;

    let mut document_scene = if let Some(input) = &args.input {
        let html = fs::read_to_string(input)
            .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
        Some(build_document_scene(&html, width, height))
    } else {
        None
    };

    let config = PlatformConfig {
        struct_size: std::mem::size_of::<PlatformConfig>() as u32,
        abi_version: PLATFORM_ABI_VERSION,
        width,
        height,
        title_utf8: title.as_ptr(),
    };

    let runtime_abi = unsafe { ffi::platform_get_abi_version() };
    if runtime_abi != PLATFORM_ABI_VERSION {
        return Err(format!(
            "platform ABI mismatch: runtime={runtime_abi}, expected={PLATFORM_ABI_VERSION}. If Zig is not installed, use headless mode."
        ));
    }

    let initialized = unsafe { ffi::platform_init_window(&config as *const PlatformConfig) };
    if initialized == PLATFORM_FALSE {
        return Err("platform_init_window returned false".to_string());
    }

    let mut renderer = Renderer::new(width, height);
    renderer.set_pattern(args.pattern);

    let mut scheduler = Scheduler::new(60).with_max_updates_per_frame(4);
    let mut last_tick = Instant::now();
    let mut running = true;

    log_info(&format!(
        "starting runtime width={width} height={height} pattern={:?} document={}",
        args.pattern,
        document_scene.is_some()
    ));

    while running {
        loop {
            let mut event = MaybeUninit::<PlatformEvent>::zeroed();
            unsafe {
                (*event.as_mut_ptr()).struct_size = std::mem::size_of::<PlatformEvent>() as u32;
            }
            let has_event = unsafe { ffi::platform_poll_event(event.as_mut_ptr()) };
            if has_event == PLATFORM_FALSE {
                break;
            }

            let event = unsafe { event.assume_init() };
            match event.kind {
                PLATFORM_EVENT_QUIT => running = false,
                PLATFORM_EVENT_KEY_DOWN if event.key_code == PLATFORM_KEY_ESCAPE => running = false,
                PLATFORM_EVENT_KEY_DOWN => {
                    let next = renderer.pattern().next();
                    renderer.set_pattern(next);
                    log_info(&format!("pattern toggled pattern={next:?}"));
                }
                PLATFORM_EVENT_RESIZE => {
                    if event.width > 0
                        && event.height > 0
                        && (event.width != width || event.height != height)
                    {
                        width = event.width;
                        height = event.height;
                        renderer.resize(width, height);
                        if let Some(scene) = &mut document_scene {
                            *scene = build_document_scene(&scene.html, width, height);
                        }
                        log_info(&format!("resized width={width} height={height}"));
                    }
                }
                _ => {}
            }
        }

        if !running {
            break;
        }

        let now = Instant::now();
        let dt = now.saturating_duration_since(last_tick);
        last_tick = now;

        let timing = scheduler.advance(dt);
        let time_seconds = timing.frame_index as f32 / 60.0;

        let overlay = OverlayInfo {
            frame_index: timing.frame_index,
            fps: timing.fps,
            width,
            height,
        };

        let framebuffer = if let Some(scene) = &document_scene {
            renderer.render_display_list(
                timing.frame_index,
                time_seconds,
                &scene.rects,
                Some(overlay),
            )
        } else {
            renderer.render_pattern(timing.frame_index, time_seconds, Some(overlay))
        };

        log_debug(&format!(
            "frame timing frame={} dt={:.4} fps={:.2} fixed_updates={}",
            timing.frame_index, timing.dt_seconds, timing.fps, timing.fixed_updates
        ));

        let frame = PlatformFrame {
            struct_size: std::mem::size_of::<PlatformFrame>() as u32,
            width,
            height,
            stride_bytes: width * 4,
            pixels_rgba8: framebuffer.as_ptr(),
        };

        let presented = unsafe { ffi::platform_present_frame(&frame as *const PlatformFrame) };
        if presented == PLATFORM_FALSE {
            running = false;
        }

        thread::sleep(Duration::from_millis(1));
    }

    unsafe { ffi::platform_shutdown() };
    Ok(())
}

fn run_headless(args: HeadlessArgs) -> Result<(), String> {
    let html = fs::read_to_string(&args.input)
        .map_err(|err| format!("failed to read {}: {err}", args.input.display()))?;

    let buffer = render_headless_buffer(&html, args.width, args.height, args.frame);

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    fs::write(&args.out, &buffer)
        .map_err(|err| format!("failed to write {}: {err}", args.out.display()))?;

    log_info(&format!(
        "headless frame written path={} width={} height={} bytes={}",
        args.out.display(),
        args.width,
        args.height,
        buffer.len()
    ));
    Ok(())
}

fn run_golden(args: GoldenArgs) -> Result<(), String> {
    fs::create_dir_all(&args.golden_dir)
        .map_err(|err| format!("failed to create {}: {err}", args.golden_dir.display()))?;

    let fixtures = collect_fixtures(&args.fixture_dir)?;
    if fixtures.is_empty() {
        return Err(format!(
            "no fixtures found in {}",
            args.fixture_dir.display()
        ));
    }

    let mut failures = Vec::new();

    for fixture in fixtures {
        let fixture_name = fixture
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("invalid fixture name: {}", fixture.display()))?;

        let html = fs::read_to_string(&fixture)
            .map_err(|err| format!("failed to read {}: {err}", fixture.display()))?;
        let buffer = render_headless_buffer(&html, args.width, args.height, args.frame);
        let hash = format!("{:016x}", fnv1a64(&buffer));

        let expected_path = args.golden_dir.join(format!("{fixture_name}.hash"));
        if args.update || !expected_path.exists() {
            fs::write(&expected_path, format!("{hash}\n")).map_err(|err| {
                format!(
                    "failed to write expected hash {}: {err}",
                    expected_path.display()
                )
            })?;
            log_info(&format!(
                "golden updated path={} hash={hash}",
                expected_path.display()
            ));
            continue;
        }

        let expected = fs::read_to_string(&expected_path)
            .map_err(|err| format!("failed to read {}: {err}", expected_path.display()))?;
        let expected = expected.trim();
        if expected != hash {
            let actual_path = args.golden_dir.join(format!("{fixture_name}.actual.hash"));
            fs::write(&actual_path, format!("{hash}\n")).map_err(|err| {
                format!(
                    "failed to write actual hash {}: {err}",
                    actual_path.display()
                )
            })?;
            failures.push(format!(
                "{} expected={} actual={} (actual hash in {})",
                fixture_name,
                expected,
                hash,
                actual_path.display()
            ));
        }
    }

    if failures.is_empty() {
        log_info(&format!(
            "golden check passed count={}",
            fixtures_len(&args.fixture_dir)?
        ));
        return Ok(());
    }

    Err(format!(
        "golden mismatches:\n{}",
        failures
            .into_iter()
            .map(|f| format!("- {f}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

fn collect_fixtures(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut fixtures = Vec::new();
    let entries =
        fs::read_dir(dir).map_err(|err| format!("failed to read {}: {err}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("failed to read fixture entry: {err}"))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("html") {
            fixtures.push(path);
        }
    }
    fixtures.sort();
    Ok(fixtures)
}

fn fixtures_len(dir: &Path) -> Result<usize, String> {
    Ok(collect_fixtures(dir)?.len())
}

fn render_headless_buffer(html: &str, width: u32, height: u32, frame: u64) -> Vec<u8> {
    let scene = build_document_scene(html, width, height);
    let mut renderer = Renderer::new(width, height);
    let overlay = OverlayInfo {
        frame_index: frame,
        fps: 0.0,
        width,
        height,
    };

    renderer
        .render_display_list(frame, frame as f32 / 60.0, &scene.rects, Some(overlay))
        .to_vec()
}

fn build_document_scene(html: &str, width: u32, height: u32) -> DocumentScene {
    let output = render_document(html, width, height);

    let mut host = StubScriptHost::default();
    if let Err(err) = host.execute(&output.scripts) {
        match err {
            ScriptError::Unsupported { script_count } => {
                log_warn(&format!(
                    "script execution unsupported in stub host script_count={script_count}"
                ));
            }
        }
    }

    let rects = display_commands_to_rects(&output.display_list.commands);
    DocumentScene {
        html: html.to_string(),
        rects,
    }
}

fn display_commands_to_rects(commands: &[DisplayCommand]) -> Vec<DrawRect> {
    commands
        .iter()
        .map(|cmd| match cmd {
            DisplayCommand::FillRect {
                x,
                y,
                width,
                height,
                color,
            } => DrawRect {
                x: *x as i32,
                y: *y as i32,
                width: *width as i32,
                height: *height as i32,
                color: *color,
            },
        })
        .collect()
}

fn parse_u32(value: &str, flag: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag}: {value}"))
}

fn next_arg(
    args: &mut std::iter::Peekable<impl Iterator<Item = String>>,
    flag: &str,
) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for b in bytes {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn log_info(message: &str) {
    eprintln!("[browser][info] {message}");
}

fn log_warn(message: &str) {
    eprintln!("[browser][warn] {message}");
}

fn log_debug(message: &str) {
    if std::env::var_os("TESSERA_DEBUG").is_some() {
        eprintln!("[browser][debug] {message}");
    }
}

#[cfg(feature = "process-split")]
fn process_split_bootstrap() {
    let mut transport = InProcessTransport::default();
    transport.send_to_content(&BrowserToContent::Tick { frame_index: 0 });
    let _ = transport.recv_for_content();
    log_debug("process-split feature enabled (ipc transport bootstrap)");
}

#[cfg(not(feature = "process-split"))]
fn process_split_bootstrap() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_run_pattern_flag() {
        let command = parse_cli(
            vec!["run", "--pattern", "rects"]
                .into_iter()
                .map(String::from),
        )
        .unwrap();
        let Command::Run(run) = command else {
            panic!("expected run command");
        };
        assert_eq!(run.pattern, Pattern::Rects);
    }

    #[test]
    fn parses_headless_required_flags() {
        let command = parse_cli(
            vec![
                "headless",
                "--input",
                "tests/fixtures/basic.html",
                "--out",
                "tests/golden/tmp.rgba",
            ]
            .into_iter()
            .map(String::from),
        )
        .unwrap();

        let Command::Headless(headless) = command else {
            panic!("expected headless command");
        };
        assert_eq!(headless.width, 960);
        assert_eq!(headless.height, 540);
    }

    #[test]
    fn converts_display_commands() {
        let commands = vec![DisplayCommand::FillRect {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
            color: [1, 2, 3, 4],
        }];

        let rects = display_commands_to_rects(&commands);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 1);
        assert_eq!(rects[0].height, 4);
    }
}
