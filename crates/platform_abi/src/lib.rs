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
pub const PLATFORM_KEY_ENTER: u32 = 13;
pub const PLATFORM_KEY_SPACE: u32 = 32;
pub const PLATFORM_KEY_F: u32 = 70;
pub const PLATFORM_KEY_H: u32 = 72;
pub const PLATFORM_KEY_J: u32 = 74;
pub const PLATFORM_KEY_K: u32 = 75;
pub const PLATFORM_KEY_S: u32 = 83;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlatformConfig {
    pub struct_size: u32,
    pub abi_version: u32,
    pub width: u32,
    pub height: u32,
    pub title_utf8: *const core::ffi::c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            struct_size: core::mem::size_of::<Self>() as u32,
            abi_version: PLATFORM_ABI_VERSION,
            width: 0,
            height: 0,
            title_utf8: core::ptr::null(),
        }
    }
}

impl Default for PlatformFrame {
    fn default() -> Self {
        Self {
            struct_size: core::mem::size_of::<Self>() as u32,
            width: 0,
            height: 0,
            stride_bytes: 0,
            pixels_rgba8: core::ptr::null(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of, MaybeUninit};

    fn offset_of_config_width() -> usize {
        let uninit = MaybeUninit::<PlatformConfig>::uninit();
        let base = uninit.as_ptr();
        // SAFETY: We compute field offsets from a dangling base pointer only.
        unsafe { (core::ptr::addr_of!((*base).width) as usize) - (base as usize) }
    }

    fn offset_of_frame_pixels() -> usize {
        let uninit = MaybeUninit::<PlatformFrame>::uninit();
        let base = uninit.as_ptr();
        // SAFETY: We compute field offsets from a dangling base pointer only.
        unsafe { (core::ptr::addr_of!((*base).pixels_rgba8) as usize) - (base as usize) }
    }

    fn offset_of_event_height() -> usize {
        let uninit = MaybeUninit::<PlatformEvent>::uninit();
        let base = uninit.as_ptr();
        // SAFETY: We compute field offsets from a dangling base pointer only.
        unsafe { (core::ptr::addr_of!((*base).height) as usize) - (base as usize) }
    }

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
        assert_eq!(offset_of_config_width(), 8);
    }

    #[test]
    fn platform_frame_layout_matches_c_abi() {
        let ptr_size = size_of::<*const u8>();
        let expected_size = if ptr_size == 8 { 24 } else { 20 };
        let expected_align = ptr_size;

        assert_eq!(size_of::<PlatformFrame>(), expected_size);
        assert_eq!(align_of::<PlatformFrame>(), expected_align);
        assert_eq!(offset_of_frame_pixels(), 16);
    }

    #[test]
    fn platform_event_layout_matches_c_abi() {
        assert_eq!(size_of::<PlatformEvent>(), 20);
        assert_eq!(align_of::<PlatformEvent>(), 4);
        assert_eq!(offset_of_event_height(), 16);
    }
}
