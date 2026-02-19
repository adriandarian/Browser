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
}
