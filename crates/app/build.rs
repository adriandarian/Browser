use std::{env, io::ErrorKind, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(platform_stub)");
    println!("cargo:rerun-if-env-changed=TESSERA_FORCE_PLATFORM_STUB");
    let target = env::var("TARGET").expect("TARGET missing");
    let profile = env::var("PROFILE").expect("PROFILE missing");

    let force_stub = env::var("TESSERA_FORCE_PLATFORM_STUB")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false);
    if force_stub {
        println!(
            "cargo:warning=TESSERA_FORCE_PLATFORM_STUB is enabled; using platform_stub for target: {target}"
        );
        println!("cargo:rustc-cfg=platform_stub");
        return;
    }

    let supported = target.contains("windows-msvc") || target.contains("apple-darwin");
    if !supported {
        println!("cargo:warning=Skipping Zig platform build for unsupported target: {target}");
        println!("cargo:rustc-cfg=platform_stub");
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
    let zig_dir = repo_root.join("zig/platform");
    let out_dir = repo_root
        .join("target/zig-out")
        .join(&target)
        .join(&profile);

    // Use ReleaseFast for non-release too, to avoid UBSan symbol dependencies from
    // Zig Debug C/ObjC builds when linking the static lib into Rust binaries.
    let optimize = "ReleaseFast";
    let zig_target = if target.contains("windows") {
        "x86_64-windows-msvc"
    } else if target.starts_with("aarch64") {
        "aarch64-macos"
    } else {
        "x86_64-macos"
    };

    let mut zig = Command::new("zig");
    zig.current_dir(&zig_dir)
        .arg("build")
        .arg(format!("-Doptimize={optimize}"))
        .arg(format!("-Dtarget={zig_target}"))
        .arg("--prefix")
        .arg(&out_dir);

    if target.contains("apple-darwin") {
        if let Ok(output) = Command::new("xcrun")
            .arg("--sdk")
            .arg("macosx")
            .arg("--show-sdk-path")
            .output()
        {
            if output.status.success() {
                let sdk_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !sdk_root.is_empty() {
                    zig.arg(format!("-Dsdk_root={sdk_root}"));
                }
            }
        }
    }

    let output = match zig.output() {
        Ok(output) => output,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            println!("cargo:warning=Zig not found; enabling platform_stub for target: {target}");
            println!("cargo:rustc-cfg=platform_stub");
            return;
        }
        Err(err) => panic!("failed to execute zig build: {err}"),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        for line in stderr.lines() {
            println!("cargo:warning=zig: {line}");
        }
        println!(
            "cargo:warning=zig build failed ({}); enabling platform_stub for target: {target}",
            output.status
        );
        println!("cargo:rustc-cfg=platform_stub");
        return;
    }

    let lib_dir = out_dir.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=platform");

    if target.contains("windows") {
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=gdi32");
    } else {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=framework=CoreGraphics");
        println!("cargo:rustc-link-lib=framework=Foundation");
    }
}
