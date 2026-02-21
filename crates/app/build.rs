use std::{env, io::ErrorKind, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rustc-check-cfg=cfg(platform_stub)");
    let target = env::var("TARGET").expect("TARGET missing");
    let profile = env::var("PROFILE").expect("PROFILE missing");

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

    let optimize = if profile == "release" {
        "ReleaseFast"
    } else {
        "Debug"
    };
    let zig_target = if target.contains("windows") {
        "x86_64-windows-msvc"
    } else if target.starts_with("aarch64") {
        "aarch64-macos"
    } else {
        "x86_64-macos"
    };

    let status = match Command::new("zig")
        .current_dir(&zig_dir)
        .arg("build")
        .arg("-Doptimize")
        .arg(optimize)
        .arg("-Dtarget")
        .arg(zig_target)
        .arg("--prefix")
        .arg(&out_dir)
        .status()
    {
        Ok(status) => status,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            println!("cargo:warning=Zig not found; enabling platform_stub for target: {target}");
            println!("cargo:rustc-cfg=platform_stub");
            return;
        }
        Err(err) => panic!("failed to execute zig build: {err}"),
    };

    if !status.success() {
        panic!("zig build failed with status {status}");
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
