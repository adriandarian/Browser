use platform_abi::{PlatformConfig, PlatformEvent, PlatformFrame};

#[cfg(not(platform_stub))]
unsafe extern "C" {
    pub fn platform_init_window(config: *const PlatformConfig) -> bool;
    pub fn platform_poll_event(out_event: *mut PlatformEvent) -> bool;
    pub fn platform_present_frame(frame: *const PlatformFrame) -> bool;
    pub fn platform_shutdown();
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_init_window(_config: *const PlatformConfig) -> bool {
    true
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_poll_event(_out_event: *mut PlatformEvent) -> bool {
    false
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_present_frame(_frame: *const PlatformFrame) -> bool {
    false
}

#[cfg(platform_stub)]
#[no_mangle]
pub unsafe extern "C" fn platform_shutdown() {}
