//! Wooting Analog SDK C-ABI definitions (plugin side), matching
//! `includes/plugin.h` and `includes/wooting-analog-sdk.h` (ABI version 2).

use std::os::raw::{c_char, c_int};

pub const ANALOG_SDK_PLUGIN_ABI_VERSION: u32 = 2;

/// `WootingAnalog_DeviceID`
pub type DeviceID = u64;

/// `WootingAnalog_DeviceType`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceType {
    Keyboard = 1,
    Keypad = 2,
    Other = 3,
}

/// `WootingAnalog_DeviceSupportLevel`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceSupportLevel {
    /// Only raw keycodes and analog values, no extra metadata.
    Limited = 0,
    /// Keycodes + analog values with metadata, position polling.
    Basic = 1,
}

/// `WootingAnalog_DeviceEventType`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceEventType {
    Connected = 1,
    Disconnected = 2,
}

/// `WootingAnalogResult` values (subset used by plugins; errors are returned
/// cast to the function's return type per the C plugin convention).
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WootingAnalogResult {
    Ok = 1,
    UnInitialized = -2000,
    NoDevices = -1999,
    DeviceDisconnected = -1998,
    Failure = -1997,
    InvalidArgument = -1996,
    NoMapping = -1993,
    IncompatibleVersion = -1991,
}

impl WootingAnalogResult {
    /// Cast a result to the float return type of `read_analog`.
    pub fn as_f32(self) -> f32 {
        (self as i32) as f32
    }

    /// Cast a result to the int return type of `initialise`/`device_info`/
    /// `read_full_buffer`.
    pub fn as_i32(self) -> c_int {
        self as i32
    }
}

/// `WootingAnalog_DeviceInfo_FFI`
///
/// The raw pointers are to plugin-owned, effectively-immutable C strings, so
/// the struct is Send+Sync despite the compiler's default for raw pointers.
#[repr(C)]
pub struct DeviceInfoFFI {
    pub vendor_id: u16,
    pub product_id: u16,
    /// NUL-terminated, owned by the plugin, stable for the struct's lifetime.
    pub manufacturer_name: *mut c_char,
    pub device_name: *mut c_char,
    pub device_id: DeviceID,
    pub device_type: DeviceType,
    pub support_level: DeviceSupportLevel,
}

unsafe impl Send for DeviceInfoFFI {}
unsafe impl Sync for DeviceInfoFFI {}

/// Callback the SDK hands to `initialise`: `(callback_data, event_type, device_info)`.
pub type DeviceEventCallback =
    unsafe extern "C" fn(*const std::ffi::c_void, DeviceEventType, *const DeviceInfoFFI);

/// FUN60 Ultra USB identity.
pub const FUN60_VID: u16 = 0x3151;
pub const FUN60_PID: u16 = 0x5030;
pub const FUN60_DEVICE_NAME: &[u8] = b"MonsGeek FUN60 Ultra\0";
pub const FUN60_MANUFACTURER: &[u8] = b"MonsGeek\0";

/// Generate a stable device ID the same way the Wooting SDK does
/// (`DefaultHasher` over vid, pid, serial bytes). Using the same algorithm
/// keeps device identity consistent with other plugins/tools.
pub fn generate_device_id(serial: &str, vendor_id: u16, product_id: u16) -> DeviceID {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;
    let mut s = DefaultHasher::new();
    s.write_u16(vendor_id);
    s.write_u16(product_id);
    s.write(serial.as_bytes());
    s.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_casts_are_negative_floats() {
        assert_eq!(WootingAnalogResult::Ok.as_f32(), 1.0);
        assert_eq!(WootingAnalogResult::UnInitialized.as_f32(), -2000.0);
        assert_eq!(WootingAnalogResult::DeviceDisconnected.as_i32(), -1998);
    }

    #[test]
    fn device_id_is_stable() {
        let a = generate_device_id("serial", 0x3151, 0x5030);
        let b = generate_device_id("serial", 0x3151, 0x5030);
        assert_eq!(a, b);
        assert_ne!(a, generate_device_id("other", 0x3151, 0x5030));
    }
}
