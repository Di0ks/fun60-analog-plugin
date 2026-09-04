//! dlopen-based test harness mimicking how the Wooting Analog SDK loads and
//! exercises the plugin. Run against a live `iot_driver depth-server` as root
//! (needs to open `/run/iot_driver/depth.sock`), e.g. `sudo -E cargo run --example dlopen_test`

use libloading::{Library, Symbol};
use std::ffi::{CStr, c_char, c_int, c_void};
use std::os::raw::c_float;

type DeviceEventCallback = unsafe extern "C" fn(
    *const c_void,
    c_int,         // DeviceEventType
    *const c_void, // DeviceInfoFFI*
);

// DeviceEventType values
#[allow(dead_code)]
const CONNECTED: c_int = 1;
#[allow(dead_code)]
const DISCONNECTED: c_int = 2;

static EVENT_LOG: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

unsafe extern "C" fn on_event(_data: *const c_void, event: c_int, info: *const c_void) {
    unsafe {
        println!("EVENT: type={event}");
        if !info.is_null() {
            // DeviceInfoFFI (repr(C), 64-bit): vid@0 u16, pid@2 u16,
            // pad@4, manufacturer ptr@8, name ptr@16, device_id u64@24,
            // device_type@32, support_level@36.
            let vid = *(info as *const u16);
            let pid = *(info.byte_add(2) as *const u16);
            let id = *(info.byte_add(24) as *const u64);
            let name_ptr = *(info.byte_add(16) as *const *const c_char);
            let name = CStr::from_ptr(name_ptr).to_string_lossy();
            println!("  device: {vid:04x}:{pid:04x} id={id:016x} name={name}");
        }
        EVENT_LOG.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

fn main() {
    let so_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/release/libfun60_analog_plugin.so".into());

    unsafe {
        let lib = Library::new(&so_path).expect("dlopen plugin");

        let name: Symbol<unsafe extern "C" fn() -> *const c_char> = lib.get(b"name").unwrap();
        println!("plugin name: {}", CStr::from_ptr(name()).to_string_lossy());

        let initialise: Symbol<unsafe extern "C" fn(*const c_void, DeviceEventCallback) -> c_int> =
            lib.get(b"initialise").unwrap();
        let rc = initialise(std::ptr::null(), on_event);
        println!("initialise -> {rc} (devices; negative = error)");

        let is_initialised: Symbol<unsafe extern "C" fn() -> bool> =
            lib.get(b"is_initialised").unwrap();
        println!("is_initialised -> {}", is_initialised());

        // device_info
        let device_info: Symbol<unsafe extern "C" fn(*mut *const c_void, c_int) -> c_int> =
            lib.get(b"device_info").unwrap();
        let mut info_ptr: *const c_void = std::ptr::null();
        let n = device_info(&mut info_ptr as *mut _ as *mut _, 1);
        println!("device_info -> {n}");
        if n == 1 && !info_ptr.is_null() {
            let vid = *(info_ptr as *const u16);
            let pid = *(info_ptr.byte_add(2) as *const u16);
            let id = *(info_ptr.byte_add(24) as *const u64);
            println!("  vid={vid:04x} pid={pid:04x} device_id={id:016x}");
        }

        // Poll loop: read_full_buffer + read_analog on a couple of keys.
        let read_full_buffer: Symbol<
            unsafe extern "C" fn(*mut u16, *mut c_float, c_int, u64) -> c_int,
        > = lib.get(b"read_full_buffer").unwrap();
        let read_analog: Symbol<unsafe extern "C" fn(u16, u64) -> c_float> =
            lib.get(b"read_analog").unwrap();

        println!("polling for 8s — press keys on the FUN60...");
        let mut codes = [0u16; 60];
        let mut vals = [0f32; 60];
        let t0 = std::time::Instant::now();
        let mut polls = 0;
        while t0.elapsed().as_secs() < 8 {
            let n = read_full_buffer(codes.as_mut_ptr(), vals.as_mut_ptr(), 60, 0);
            polls += 1;
            if n > 0 {
                for i in 0..n as usize {
                    println!("  pressed: HID {:#04x} = {:.3}", codes[i], vals[i]);
                }
            }
            // Spot-check read_analog on A (0x04) and an unmapped code.
            let a = read_analog(0x04, 0);
            if a > 0.0 {
                println!("  read_analog(A=0x04) = {a:.3}");
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        println!(
            "done: {polls} polls, events fired: {}",
            EVENT_LOG.load(std::sync::atomic::Ordering::Relaxed)
        );
    }
}
