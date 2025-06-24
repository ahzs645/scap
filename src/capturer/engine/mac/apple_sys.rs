#![allow(non_upper_case_globals)]


use core_media_rs::cm_time::CMTime;

pub type Float64 = f64;

// Core Foundation types
pub type CFTypeRef = *const std::ffi::c_void;
pub type CFDictionaryRef = *const std::ffi::c_void;
pub type CFNumberRef = *const std::ffi::c_void;
pub type CFNumberType = u32;

// Core Foundation constants
pub const CFNumberType_kCFNumberSInt64Type: CFNumberType = 4;

// Screen capture frame status constants
pub const SCFrameStatus_SCFrameStatusComplete: i64 = 0;

// Core Foundation dictionary key
pub struct SCStreamFrameInfoStatus(pub *const std::ffi::c_void);
unsafe impl Send for SCStreamFrameInfoStatus {}
unsafe impl Sync for SCStreamFrameInfoStatus {}

#[allow(non_upper_case_globals)]
pub static SC_STREAM_FRAME_INFO_STATUS: SCStreamFrameInfoStatus = SCStreamFrameInfoStatus(std::ptr::null());

extern "C" {
    // Core Foundation functions
    pub fn CFDictionaryGetValue(theDict: CFDictionaryRef, key: *const std::ffi::c_void) -> CFTypeRef;
    pub fn CFNumberGetValue(number: CFNumberRef, theType: CFNumberType, valuePtr: *mut std::ffi::c_void) -> u8;
    
    // Core Media functions
    pub fn CMTimeGetSeconds(time: CMTime) -> Float64;
}
