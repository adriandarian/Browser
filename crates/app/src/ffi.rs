use platform_abi::{PlatformConfig, PlatformEvent, PlatformFrame};

#[cfg(not(platform_stub))]
unsafe extern "C" {
    pub fn platform_get_abi_version() -> u32;
    pub fn platform_init_window(config: *const PlatformConfig) -> u8;
    pub fn platform_poll_event(out_event: *mut PlatformEvent) -> u8;
    pub fn platform_present_frame(frame: *const PlatformFrame) -> u8;
    pub fn platform_shutdown();
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_get_abi_version() -> u32 {
    0
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_init_window(_config: *const PlatformConfig) -> u8 {
    0
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_poll_event(_out_event: *mut PlatformEvent) -> u8 {
    0
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_present_frame(_frame: *const PlatformFrame) -> u8 {
    0
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_shutdown() {}
