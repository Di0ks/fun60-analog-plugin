//! Plugin core: owns the UDS connection and exposes the state the C ABI
//! layer reads. SDK interaction model is poll-based (`read_analog` /
//! `read_full_buffer`), so the reader thread just keeps the cache fresh.

use crate::ffi::{
    DeviceEventCallback, DeviceEventType, DeviceID, DeviceInfoFFI, DeviceSupportLevel, DeviceType,
    FUN60_DEVICE_NAME, FUN60_MANUFACTURER, FUN60_PID, FUN60_VID, generate_device_id,
};
use crate::keymap::{DEFAULT_FULL_TRAVEL, build_matrix_to_hid, normalize};
use crate::socket::{Connection, SOCKET_PATH_ENV};
use std::collections::HashMap;
use std::ffi::CString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Default socket path (kept in sync with depth_server).
pub const DEFAULT_SOCKET_PATH: &str = "/run/iot_driver/depth.sock";

/// Serial used for the device ID. The depth protocol does not expose a
/// serial; fall back to a stable placeholder tied to the VID/PID identity.
const SERIAL_PLACEHOLDER: &str = "fun60-ultra";

/// Full travel override from the environment (0.01mm units).
fn full_travel_from_env() -> u16 {
    std::env::var("FUN60_FULL_TRAVEL")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&v| v > 0)
        .unwrap_or(DEFAULT_FULL_TRAVEL)
}

pub fn socket_path() -> PathBuf {
    std::env::var(SOCKET_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_SOCKET_PATH))
}

/// Shared plugin state accessible from the C ABI exports.
///
/// Interior mutability only: the SDK holds this behind `Arc`/static and all
/// access is from `extern "C"` fns taking `&self`.
pub struct PluginState {
    connection: Mutex<Option<Connection>>,
    /// matrix index -> HID code
    pub keymap: HashMap<u8, u16>,
    /// Full travel in depth_raw units for normalization (env-overridable).
    pub full_travel: u16,
    device_info: DeviceInfoFFI,
    /// Owns the memory `device_info`'s char pointers point into. Must outlive
    /// the struct (it does: self lives in a static OnceLock).
    _device_name_c: CString,
    _manufacturer_c: CString,
    device_id: DeviceID,
    /// Callback + data from the SDK, used to fire connect/disconnect events.
    callback: Mutex<Option<(usize, DeviceEventCallback)>>,
    /// Set while a device-event callback is in flight (guards re-entrancy).
    notifying: AtomicBool,
}

impl PluginState {
    /// Test constructor with a forced connection state (no socket).
    pub fn new_for_test(connection: Option<Connection>) -> Self {
        let st = Self::new();
        // Replace whatever connection attempt produced.
        *st.connection.lock().unwrap() = connection;
        st
    }

    /// Create state, attempting an initial connection (failure tolerated:
    /// the plugin can load before the driver is running and reconnect later).
    ///
    /// This is the `Default` impl; kept as `new` for clarity at call sites.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let connection = Connection::connect(&socket_path()).ok();
        let device_name_c = CString::new(
            FUN60_DEVICE_NAME
                .iter()
                .copied()
                .take_while(|&b| b != 0)
                .collect::<Vec<u8>>(),
        )
        .expect("no interior NUL");
        let manufacturer_c = CString::new(
            FUN60_MANUFACTURER
                .iter()
                .copied()
                .take_while(|&b| b != 0)
                .collect::<Vec<u8>>(),
        )
        .expect("no interior NUL");
        let device_id = generate_device_id(SERIAL_PLACEHOLDER, FUN60_VID, FUN60_PID);
        Self {
            connection: Mutex::new(connection),
            keymap: build_matrix_to_hid().into_iter().collect(),
            full_travel: full_travel_from_env(),
            device_info: DeviceInfoFFI {
                vendor_id: FUN60_VID,
                product_id: FUN60_PID,
                manufacturer_name: manufacturer_c.as_ptr() as *mut _,
                device_name: device_name_c.as_ptr() as *mut _,
                device_id,
                device_type: DeviceType::Keyboard,
                support_level: DeviceSupportLevel::Basic,
            },
            _device_name_c: device_name_c,
            _manufacturer_c: manufacturer_c,
            device_id,
            callback: Mutex::new(None),
            notifying: AtomicBool::new(false),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connection
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.state.is_connected())
            .unwrap_or(false)
    }

    /// Connect if not already connected; fires the SDK Connected event on a
    /// successful (re)connect. Returns true when connected afterwards.
    pub fn ensure_connected(&self) -> bool {
        if self.is_connected() {
            return true;
        }
        let mut guard = self.connection.lock().unwrap();
        // Re-check under the lock (another thread may have reconnected).
        if let Some(conn) = guard.as_ref()
            && conn.state.is_connected()
        {
            return true;
        }
        match Connection::connect(&socket_path()) {
            Ok(conn) => {
                *guard = Some(conn);
                drop(guard);
                self.notify_device_event(DeviceEventType::Connected);
                true
            }
            Err(e) => {
                eprintln!("fun60 plugin: reconnect failed: {e}");
                false
            }
        }
    }

    /// Drop the connection, firing a disconnect event if it was up.
    pub fn disconnect(&self) {
        let had = {
            let mut guard = self.connection.lock().unwrap();
            let had = guard
                .as_ref()
                .map(|c| c.state.is_connected())
                .unwrap_or(false);
            if let Some(conn) = guard.take() {
                conn.shutdown();
            }
            had
        };
        if had {
            self.notify_device_event(DeviceEventType::Disconnected);
        }
    }

    /// Normalized analog value for a matrix index (0.0 if at rest/unknown).
    pub fn analog(&self, matrix_index: u8) -> f32 {
        let guard = self.connection.lock().unwrap();
        guard
            .as_ref()
            .map(|c| normalize(c.state.get(matrix_index), self.full_travel))
            .unwrap_or(0.0)
    }

    /// All pressed keys as (HID code, normalized value), skipping unmapped.
    pub fn pressed_analog(&self) -> Vec<(u16, f32)> {
        let guard = self.connection.lock().unwrap();
        let conn = match guard.as_ref() {
            Some(c) => c,
            None => return Vec::new(),
        };
        conn.state
            .pressed()
            .iter()
            .filter_map(|&(idx, depth)| {
                self.keymap
                    .get(&idx)
                    .map(|&code| (code, normalize(depth, self.full_travel)))
            })
            .filter(|&(_, v)| v > 0.0)
            .collect()
    }

    /// Fire the SDK callback for a connect/disconnect event. Re-entrant
    /// calls are dropped to avoid deadlocking on the SDK side.
    pub fn notify_device_event(&self, event: DeviceEventType) {
        if self.notifying.swap(true, Ordering::SeqCst) {
            return;
        }
        let guard = self.callback.lock().unwrap();
        if let Some((data, cb)) = guard.as_ref() {
            unsafe {
                cb(
                    *data as *const std::ffi::c_void,
                    event,
                    &self.device_info as *const DeviceInfoFFI,
                );
            }
        }
        self.notifying.store(false, Ordering::SeqCst);
    }

    pub fn set_callback(&self, data: usize, cb: DeviceEventCallback) {
        *self.callback.lock().unwrap() = Some((data, cb));
    }

    pub fn clear_callback(&self) {
        *self.callback.lock().unwrap() = None;
    }

    pub fn device_id(&self) -> DeviceID {
        self.device_id
    }

    /// Pointer to the retained device-info struct (memory is stable: the
    /// CStrings live in self, self lives in a static OnceLock).
    pub fn device_info_ptr(&self) -> *const DeviceInfoFFI {
        &self.device_info as *const DeviceInfoFFI
    }
}

/// Global plugin state, created lazily on first `initialise`/read call.
pub static PLUGIN: OnceLock<PluginState> = OnceLock::new();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_without_connection_reports_zero() {
        let st = PluginState::new_for_test(None);
        assert!(!st.is_connected());
        assert_eq!(st.analog(9), 0.0);
        assert!(st.pressed_analog().is_empty());
        assert_eq!(st.keymap.get(&9), Some(&0x04));
    }

    #[test]
    fn device_info_fields_populated() {
        let st = PluginState::new_for_test(None);
        assert_eq!(st.device_info.vendor_id, 0x3151);
        assert_eq!(st.device_info.product_id, 0x5030);
        assert_eq!(st.device_info.device_id, st.device_id());
        unsafe {
            assert_eq!(
                std::ffi::CStr::from_ptr(st.device_info.device_name)
                    .to_str()
                    .unwrap(),
                "MonsGeek FUN60 Ultra"
            );
        }
    }
}
