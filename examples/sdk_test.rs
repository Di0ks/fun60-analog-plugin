//! Phase 5 verification: exercise the REAL Wooting Analog SDK
//! (libwooting_analog_sdk.so) — it scans /usr/local/share/WootingAnalogPlugins
//! and should pick up the installed FUN60 plugin.
//!
//! Run AFTER `sudo ./install.sh` and with `iot_driver depth-server` running.
//! The SDK is loaded with libloading (not link-time) so the example builds
//! without a build.rs; symbols resolve through the loaded library.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_float, c_int, c_uint, c_void};
use std::os::raw::c_ulong;

type DeviceEventCb = unsafe extern "C" fn(
    *const c_void,
    c_int,         // WootingAnalog_DeviceEventType (1 connected, 2 disconnected)
    *const c_void, // WootingAnalog_DeviceInfo_FFI*
);

// WootingAnalogResult
const WOOTING_OK: c_int = 1;

// WootingAnalog_KeycodeType
const WOOTING_KEYCODE_HID: c_uint = 0;

static EVENTS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn on_device_event(_data: *const c_void, event: c_int, info: *const c_void) {
    unsafe {
        EVENTS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let kind = match event {
            1 => "Connected",
            2 => "Disconnected",
            _ => "?",
        };
        if !info.is_null() {
            // DeviceInfoFFI (64-bit): vid@0, pid@2, name ptr@16, id u64@24
            let vid = *(info as *const u16);
            let pid = *(info.byte_add(2) as *const u16);
            let id = *(info.byte_add(24) as *const u64);
            let name_ptr = *(info.byte_add(16) as *const *const c_char);
            let name = std::ffi::CStr::from_ptr(name_ptr).to_string_lossy();
            println!("SDK device event: {kind}: {vid:04x}:{pid:04x} '{name}' id={id:016x}");
        } else {
            println!("SDK device event: {kind}");
        }
    }
}

fn main() {
    let sdk = unsafe { Library::new("libwooting_analog_sdk.so") }.expect(
        "load libwooting_analog_sdk.so (is wooting-analog-sdk-bin installed and in ldconfig?)",
    );

    unsafe {
        let initialise: Symbol<unsafe extern "C" fn() -> c_int> =
            sdk.get(b"wooting_analog_initialise").unwrap();
        let uninitialise: Symbol<unsafe extern "C" fn() -> c_int> =
            sdk.get(b"wooting_analog_uninitialise").unwrap();
        let is_initialised: Symbol<unsafe extern "C" fn() -> bool> =
            sdk.get(b"wooting_analog_is_initialised").unwrap();
        let set_keycode_mode: Symbol<unsafe extern "C" fn(c_uint) -> c_int> =
            sdk.get(b"wooting_analog_set_keycode_mode").unwrap();
        let read_full_buffer: Symbol<
            unsafe extern "C" fn(*mut u16, *mut c_float, c_ulong) -> c_int,
        > = sdk.get(b"wooting_analog_read_full_buffer").unwrap();
        let set_device_event_cb: Symbol<
            unsafe extern "C" fn(DeviceEventCb, *const c_void) -> c_int,
        > = sdk.get(b"wooting_analog_set_device_event_cb").unwrap();
        let get_devices: Symbol<unsafe extern "C" fn(*mut *const c_void, c_ulong) -> c_int> = sdk
            .get(b"wooting_analog_get_connected_devices_info")
            .unwrap();

        println!("wooting_analog_initialise...");
        let rc = initialise();
        println!("  -> {rc} (1 = WOOTING_OK, -1995 = NoPlugins)");
        if rc != WOOTING_OK {
            println!(
                "SDK init failed — is the plugin installed in /usr/local/share/WootingAnalogPlugins/fun60_analog_plugin/?"
            );
            return;
        }
        println!("is_initialised: {}", is_initialised());

        println!(
            "set_keycode_mode(HID) -> {}",
            set_keycode_mode(WOOTING_KEYCODE_HID)
        );
        println!(
            "set_device_event_cb -> {}",
            set_device_event_cb(on_device_event, std::ptr::null())
        );

        let mut infos: [*const c_void; 8] = [std::ptr::null(); 8];
        let n = get_devices(infos.as_mut_ptr(), 8);
        println!("connected devices: {n}");
        for &info in infos.iter().take(n as usize) {
            if info.is_null() {
                continue;
            }
            let vid = *(info as *const u16);
            let pid = *(info.byte_add(2) as *const u16);
            let id = *(info.byte_add(24) as *const u64);
            println!("  {vid:04x}:{pid:04x} id={id:016x}");
        }

        println!("polling SDK read_full_buffer for 10s — press keys on the FUN60...");
        let mut codes = [0u16; 60];
        let mut vals = [0f32; 60];
        let t0 = std::time::Instant::now();
        let mut readings = 0usize;
        while t0.elapsed().as_secs() < 10 {
            let n = read_full_buffer(codes.as_mut_ptr(), vals.as_mut_ptr(), 60);
            if n > 0 {
                readings += n as usize;
                for i in 0..n as usize {
                    println!("  pressed: HID {:#04x} = {:.3}", codes[i], vals[i]);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        println!(
            "done. pressed readings: {readings}, device events: {}",
            EVENTS.load(std::sync::atomic::Ordering::Relaxed)
        );
        println!("uninitialise -> {}", uninitialise());
    }
}
