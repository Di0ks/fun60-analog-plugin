//! Live test of Connection::subscribe(): connect, receive the initial
//! snapshot, then forward events as they arrive. Run against a depth server.

use fun60_analog_plugin::socket::{Connection, DepthUpdate, SOCKET_PATH_ENV};
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn main() {
    let path = std::env::var(SOCKET_PATH_ENV)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/test-depth.sock"));

    let conn = Connection::connect(&path).expect("connect");
    let rx = conn.subscribe();
    println!("subscribed; queue len at subscribe: {}", rx.len());

    // Drain the initial snapshot.
    let initial = rx.recv_timeout(Duration::from_secs(1));
    match &initial {
        Some(DepthUpdate::Snapshot(entries)) => {
            println!("initial snapshot: {} entries", entries.len());
        }
        other => println!("unexpected first update: {other:?}"),
    }

    println!("streaming events for 8s (press keys!)...");
    let mut events = 0usize;
    let mut snapshots = 0usize;
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Some(DepthUpdate::Event(e)) => {
                if events < 8 {
                    println!("  event: key {:3} depth {}", e.key_index, e.depth_raw);
                }
                events += 1;
            }
            Some(DepthUpdate::Snapshot(entries)) => {
                snapshots += 1;
                println!("  snapshot: {} entries", entries.len());
            }
            Some(DepthUpdate::Disconnected) => {
                println!("  DISCONNECTED");
                break;
            }
            None => {}
        }
    }
    println!("done: {events} events, {snapshots} snapshots in 8s");
}
