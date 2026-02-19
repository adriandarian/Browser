#![forbid(unsafe_op_in_unsafe_fn)]

pub const PLATFORM_ABI_VERSION: u32 = 2;

pub const PLATFORM_FALSE: u8 = 0;
pub const PLATFORM_TRUE: u8 = 1;

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
    pub struct_size: u32,
    pub abi_version: u32,
    pub width: u32,
    pub height: u32,
    pub title_utf8: *const core::ffi::c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformFrame {
    pub struct_size: u32,
    pub width: u32,
    pub height: u32,
    pub stride_bytes: u32,
    pub pixels_rgba8: *const u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformEvent {
    pub struct_size: u32,
    pub kind: u32,
    pub key_code: u32,
    pub width: u32,
    pub height: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn abi_constants_match_contract() {
        assert_eq!(PLATFORM_ABI_VERSION, 2);
        assert_eq!(PLATFORM_FALSE, 0);
        assert_eq!(PLATFORM_TRUE, 1);
    }

    #[test]
    fn platform_config_layout_matches_c_abi() {
        let ptr_size = size_of::<*const core::ffi::c_char>();
        let expected_size = if ptr_size == 8 { 24 } else { 20 };
        let expected_align = ptr_size;

        assert_eq!(size_of::<PlatformConfig>(), expected_size);
        assert_eq!(align_of::<PlatformConfig>(), expected_align);
    }

    #[test]
    fn platform_frame_layout_matches_c_abi() {
        let ptr_size = size_of::<*const u8>();
        let expected_size = if ptr_size == 8 { 24 } else { 20 };
        let expected_align = ptr_size;

        assert_eq!(size_of::<PlatformFrame>(), expected_size);
        assert_eq!(align_of::<PlatformFrame>(), expected_align);
    }

    #[test]
    fn platform_event_layout_matches_c_abi() {
        assert_eq!(size_of::<PlatformEvent>(), 20);
        assert_eq!(align_of::<PlatformEvent>(), 4);
    }
}
