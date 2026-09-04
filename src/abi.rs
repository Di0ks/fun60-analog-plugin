//! C ABI exports for the Wooting Analog SDK (`includes/plugin.h`, ABI v2).

use crate::ffi::{self, WootingAnalogResult};
use crate::state::{PLUGIN, PluginState};

/// Plugin name reported to the SDK.
const PLUGIN_NAME: &[u8] = b"FUN60 Ultra Analog\0";

/// `const char *name()`
#[unsafe(no_mangle)]
pub extern "C" fn name() -> *const std::os::raw::c_char {
    PLUGIN_NAME.as_ptr() as *const std::os::raw::c_char
}

/// `int initialise(void const *callback_data, device_event callback)`
///
/// Returns the number of connected devices (1 with the FUN60 reachable via
/// the depth server), 0 if the device is not (yet) reachable, or a negative
/// `WootingAnalogResult` cast to int on failure.
#[unsafe(no_mangle)]
pub extern "C" fn initialise(
    callback_data: *const std::ffi::c_void,
    callback: ffi::DeviceEventCallback,
) -> std::os::raw::c_int {
    let plugin = PLUGIN.get_or_init(PluginState::new);
    plugin.set_callback(callback_data as usize, callback);

    if plugin.ensure_connected() {
        1
    } else {
        // The plugin loaded fine; the SDK can retry via later read calls
        // (each one attempts reconnection) or expect a Connected event.
        WootingAnalogResult::NoDevices.as_i32()
    }
}

/// `bool is_initialised()`
#[unsafe(no_mangle)]
pub extern "C" fn is_initialised() -> bool {
    PLUGIN.get().map(|p| p.is_connected()).unwrap_or(false)
}

/// `void unload()`
#[unsafe(no_mangle)]
pub extern "C" fn unload() {
    if let Some(p) = PLUGIN.get() {
        p.clear_callback();
        p.disconnect();
    }
}

/// `int device_info(const WootingAnalog_DeviceInfo_FFI *buffer[], int len)`
///
/// Writes to `buffer` are part of the plugin.h contract: the SDK passes a
/// valid out-array and length. Pointer dereference is therefore safe per ABI.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn device_info(
    buffer: *mut *const ffi::DeviceInfoFFI,
    len: std::os::raw::c_int,
) -> std::os::raw::c_int {
    let Some(p) = PLUGIN.get() else {
        return WootingAnalogResult::UnInitialized.as_i32();
    };
    if !p.is_connected() && !p.ensure_connected() {
        return WootingAnalogResult::DeviceDisconnected.as_i32();
    }
    if len < 1 {
        return 0;
    }
    unsafe {
        *buffer = p.device_info_ptr();
    }
    1
}

/// `float read_analog(uint16_t code, WootingAnalog_DeviceID device)`
#[unsafe(no_mangle)]
pub extern "C" fn read_analog(code: u16, device: ffi::DeviceID) -> f32 {
    let Some(p) = PLUGIN.get() else {
        return WootingAnalogResult::UnInitialized.as_f32();
    };
    if device != 0 && device != p.device_id() {
        return WootingAnalogResult::NoDevices.as_f32();
    }
    if !p.is_connected() && !p.ensure_connected() {
        return WootingAnalogResult::DeviceDisconnected.as_f32();
    }
    // Invert the keymap: HID code -> matrix index (reverse lookup).
    let idx = p
        .keymap
        .iter()
        .find_map(|(&i, &c)| (c == code).then_some(i));
    match idx {
        Some(i) => p.analog(i),
        None => WootingAnalogResult::NoMapping.as_f32(),
    }
}

/// `int read_full_buffer(uint16_t code_buffer[], float analog_buffer[], int len, WootingAnalog_DeviceID device)`
///
/// Writes to `code_buffer`/`analog_buffer` are part of the plugin.h contract
/// (SDK passes arrays of at least `len`). Safe per ABI.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn read_full_buffer(
    code_buffer: *mut u16,
    analog_buffer: *mut f32,
    len: std::os::raw::c_int,
    device: ffi::DeviceID,
) -> std::os::raw::c_int {
    let Some(p) = PLUGIN.get() else {
        return WootingAnalogResult::UnInitialized.as_i32();
    };
    if device != 0 && device != p.device_id() {
        return WootingAnalogResult::NoDevices.as_i32();
    }
    if !p.is_connected() && !p.ensure_connected() {
        return WootingAnalogResult::DeviceDisconnected.as_i32();
    }
    if len < 0 {
        return WootingAnalogResult::InvalidArgument.as_i32();
    }
    let pressed = p.pressed_analog();
    let n = pressed.len().min(len as usize);
    unsafe {
        for (i, (code, value)) in pressed.iter().take(n).enumerate() {
            *code_buffer.add(i) = *code;
            *analog_buffer.add(i) = *value;
        }
    }
    n as std::os::raw::c_int
}
