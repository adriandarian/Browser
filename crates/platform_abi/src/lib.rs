#![forbid(unsafe_op_in_unsafe_fn)]

pub const PLATFORM_ABI_VERSION: u32 = 1;

pub const PLATFORM_EVENT_NONE: u32 = 0;
pub const PLATFORM_EVENT_QUIT: u32 = 1;
pub const PLATFORM_EVENT_KEY_DOWN: u32 = 2;
pub const PLATFORM_EVENT_KEY_UP: u32 = 3;
pub const PLATFORM_EVENT_RESIZE: u32 = 4;

pub const PLATFORM_KEY_UNKNOWN: u32 = 0;
pub const PLATFORM_KEY_ESCAPE: u32 = 27;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformConfig {
    pub abi_version: u32,
    pub width: u32,
    pub height: u32,
    pub title_utf8: *const core::ffi::c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformFrame {
    pub width: u32,
    pub height: u32,
    pub stride_bytes: u32,
    pub pixels_rgba8: *const u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformEvent {
    pub kind: u32,
    pub key_code: u32,
    pub width: u32,
    pub height: u32,
}
