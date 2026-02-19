const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const lib = b.addStaticLibrary(.{
        .name = "platform",
        .target = target,
        .optimize = optimize,
    });

    lib.addIncludePath(b.path("../../include"));
    lib.linkLibC();

    const os_tag = target.result.os.tag;
    if (os_tag == .windows) {
        lib.addCSourceFile(.{ .file = b.path("src/platform_windows.c"), .flags = &.{} });
        lib.linkSystemLibrary("user32");
        lib.linkSystemLibrary("gdi32");
    } else if (os_tag == .macos) {
        lib.addCSourceFile(.{ .file = b.path("src/platform_macos.m"), .flags = &.{ "-fobjc-arc" } });
        lib.linkFramework("AppKit");
        lib.linkFramework("CoreGraphics");
        lib.linkFramework("Foundation");
    } else {
        lib.addCSourceFile(.{ .file = b.path("src/platform_stub.c"), .flags = &.{} });
    }

    b.installArtifact(lib);

    const dump_symbols = b.addSystemCommand(&.{ "sh", "-c", "nm -g --defined-only \"$1\" | awk '{print $3}' > \"$2\"", "_" });
    dump_symbols.addFileArg(lib.getEmittedBin());
    const symbols_path = dump_symbols.addOutputFileArg("platform_symbols.txt");

    const check_symbols = b.addSystemCommand(&.{
        "sh",
        "-c",
        "for sym in platform_get_abi_version platform_init_window platform_poll_event platform_present_frame platform_shutdown; do grep -Fx \"$sym\" \"$1\" >/dev/null || { echo \"missing symbol: $sym\"; exit 1; }; done",
        "_",
    });
    check_symbols.addFileArg(symbols_path);

    const abi_symbols_step = b.step("abi-symbols", "Export and verify ABI symbol list");
    abi_symbols_step.dependOn(&dump_symbols.step);
    abi_symbols_step.dependOn(&check_symbols.step);
}
