use fun60_analog_plugin::keymap::{DEFAULT_FULL_TRAVEL, build_matrix_to_hid, normalize};
use fun60_analog_plugin::socket::{Connection, SOCKET_PATH_ENV};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    let path = std::env::var(SOCKET_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/test-depth.sock"));

    let conn = Connection::connect(&path).expect("connect to depth server");
    println!("connected; waiting for events for 8s (press keys!)...");

    let table: HashMap<u8, u16> = build_matrix_to_hid().into_iter().collect();
    let mut events = 0usize;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        for (idx, depth) in conn.state.pressed() {
            let code = table.get(&idx).copied().unwrap_or(0x0200 + idx as u16);
            println!(
                "matrix {:3} -> HID {:#04x}  depth {:3} ({:.2})",
                idx,
                code,
                depth,
                normalize(depth, DEFAULT_FULL_TRAVEL)
            );
            events += 1;
            if events > 40 {
                println!("... (stopping early)");
                return;
            }
        }
        std::thread::sleep(Duration::from_millis(300));
    }
    println!("total observed pressed-key states: {events}");
}
