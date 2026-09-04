//! UDS client for `iot_driver depth-server`.
//!
//! Connects to the driver's Unix domain socket, parses the binary frame
//! protocol, and maintains a last-known-depth cache.
//!
//! Server frames (little-endian):
//! - `S` snapshot: u8 entry_count, then entry_count × (u8 key_index, u16 depth_raw)
//! - `D` event:    u8 key_index, u16 depth_raw
//! - `E` error:    u8 error_code
//!
//! Client commands: `P` ping, `Q` query, `X` close.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Default socket path (matches depth_server::DEFAULT_SOCKET_PATH).
pub const DEFAULT_SOCKET_PATH: &str = "/run/iot_driver/depth.sock";

/// Env var overriding the socket path.
pub const SOCKET_PATH_ENV: &str = "IOT_DRIVER_DEPTH_SOCK";

/// One depth reading.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthEvent {
    pub key_index: u8,
    pub depth_raw: u16,
}

/// A queued event for subscribers: a per-key depth update, a full snapshot
/// (on connect / reconnect / explicit query), or a connection-down marker.
#[derive(Debug, Clone, PartialEq)]
pub enum DepthUpdate {
    /// One key's depth changed.
    Event(DepthEvent),
    /// Full state: all keys' last-known depths (keys absent are at rest).
    Snapshot(Vec<DepthEvent>),
    /// The connection to the depth server went down.
    Disconnected,
}

/// Default per-subscriber queue capacity.
pub const DEFAULT_QUEUE_CAPACITY: usize = 1024;

/// Shared state for one subscriber: the event ring + its condvar.
type RingShared = Arc<(Mutex<EventRing>, std::sync::Condvar)>;
/// Producer-side subscriber registry.
type SubscriberList = Arc<Mutex<Vec<RingShared>>>;

/// Bounded drop-oldest event queue shared between the reader thread (producer)
/// and a subscriber (consumer).
struct EventRing {
    deque: std::collections::VecDeque<DepthUpdate>,
    capacity: usize,
    /// Set when the reader thread exits: wakes recv and signals closure.
    closed: bool,
}

/// Receiver side of a subscription.
pub struct EventReceiver {
    inner: RingShared,
}

/// Blocking iterator over events from [`EventReceiver`].
pub struct EventIter {
    receiver: EventReceiver
}

impl Iterator for EventIter {
    type Item = DepthUpdate;

    /// Blocks until the next [`DepthUpdate`] is available.
    /// 
    /// Returns `None` if the event queue is closed.
    fn next(&mut self) -> Option<Self::Item> {
        let (lock, cvar) = &*self.receiver.inner;
        let mut ring = lock.lock().unwrap();
        loop {
            if let Some(ev) = ring.deque.pop_front() {
                return Some(ev);
            }
            if ring.closed {
                return None;
            }
            let guard = cvar.wait(ring).unwrap();
            ring = guard;
        }
    }
}

impl IntoIterator for EventReceiver {
    type Item = DepthUpdate;

    type IntoIter = EventIter;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            receiver: self
        }
    }
}

impl EventReceiver {
    /// Blocking receive with timeout.
    pub fn recv_timeout(&self, timeout: Duration) -> Option<DepthUpdate> {
        let (lock, cvar) = &*self.inner;
        let mut ring = lock.lock().unwrap();
        loop {
            if let Some(ev) = ring.deque.pop_front() {
                return Some(ev);
            }
            if ring.closed {
                return None;
            }
            let (guard, result) = cvar.wait_timeout(ring, timeout).unwrap();
            ring = guard;
            if result.timed_out() && ring.deque.is_empty() && !ring.closed {
                return None;
            }
        }
    }

    /// Non-blocking receive.
    pub fn try_recv(&self) -> Option<DepthUpdate> {
        let (lock, _) = &*self.inner;
        let mut ring = lock.lock().unwrap();
        ring.deque.pop_front()
    }

    /// Number of queued events.
    pub fn len(&self) -> usize {
        self.inner.0.lock().unwrap().deque.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Drop for EventReceiver {
    fn drop(&mut self) {
        // Removing the receiver from the producer's list is handled lazily:
        // the reader skips closed rings. Nothing to do here beyond letting
        // the Arc drop.
    }
}

/// Errors from the socket layer.
#[derive(Debug)]
pub enum SocketError {
    /// Cannot connect to the depth server.
    Connect(std::io::Error),
    /// Connection lost mid-session.
    Disconnected(std::io::Error),
    /// Server sent an unparseable frame.
    Protocol(&'static str),
}

impl std::fmt::Display for SocketError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SocketError::Connect(e) => write!(f, "connect failed: {e}"),
            SocketError::Disconnected(e) => write!(f, "disconnected: {e}"),
            SocketError::Protocol(m) => write!(f, "protocol error: {m}"),
        }
    }
}

impl std::error::Error for SocketError {}

/// Shared depth state updated by the reader thread.
#[derive(Default)]
pub struct DepthState {
    depths: Mutex<HashMap<u8, u16>>,
    connected: AtomicBool,
}

impl DepthState {
    /// Current last-known depth for a key (0 if never reported).
    pub fn get(&self, key_index: u8) -> u16 {
        self.depths
            .lock()
            .unwrap()
            .get(&key_index)
            .copied()
            .unwrap_or(0)
    }

    /// Snapshot of all last-known depths > 0.
    pub fn pressed(&self) -> Vec<(u8, u16)> {
        self.depths
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, d)| **d > 0)
            .map(|(&k, &d)| (k, d))
            .collect()
    }

    /// Snapshot of ALL last-known depths (including 0 / at rest).
    pub fn snapshot_all(&self) -> Vec<(u8, u16)> {
        self.depths
            .lock()
            .unwrap()
            .iter()
            .map(|(&k, &d)| (k, d))
            .collect()
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    fn set_connected(&self, up: bool) {
        self.connected.store(up, Ordering::Relaxed);
    }

    fn apply(&self, key_index: u8, depth_raw: u16) {
        self.depths.lock().unwrap().insert(key_index, depth_raw);
    }

    fn clear(&self) {
        self.depths.lock().unwrap().clear();
    }
}

/// Parse a contiguous buffer of server frames into events.
///
/// Returns parsed events; unknown frame types are skipped (one byte + payload
/// length is deducible per type, so the stream stays aligned). Returns a
/// Protocol error if a frame header implies a truncated buffer.
pub fn parse_frames(buf: &[u8]) -> Result<Vec<Frame>, SocketError> {
    let mut frames = Vec::new();
    let mut off = 0;
    while off < buf.len() {
        match buf[off] {
            b'D' if off + 4 <= buf.len() => {
                frames.push(Frame::Event(DepthEvent {
                    key_index: buf[off + 1],
                    depth_raw: u16::from_le_bytes([buf[off + 2], buf[off + 3]]),
                }));
                off += 4;
            }
            b'S' if off + 2 <= buf.len() => {
                let count = buf[off + 1] as usize;
                let end = off + 2 + count * 3;
                if end > buf.len() {
                    return Err(SocketError::Protocol("truncated snapshot"));
                }
                let mut entries = Vec::with_capacity(count);
                for i in 0..count {
                    let base = off + 2 + i * 3;
                    entries.push(DepthEvent {
                        key_index: buf[base],
                        depth_raw: u16::from_le_bytes([buf[base + 1], buf[base + 2]]),
                    });
                }
                frames.push(Frame::Snapshot(entries));
                off = end;
            }
            b'E' if off + 2 <= buf.len() => {
                frames.push(Frame::Error(buf[off + 1]));
                off += 2;
            }
            _ => return Err(SocketError::Protocol("unknown or truncated frame")),
        }
    }
    Ok(frames)
}

/// A parsed server frame.
#[derive(Debug, PartialEq)]
pub enum Frame {
    Snapshot(Vec<DepthEvent>),
    Event(DepthEvent),
    Error(u8),
}

/// Live connection to the depth server: spawns a reader thread that keeps
/// `state` up to date. `JoinHandle`-less; the thread exits on disconnect or
/// when the shared `stop` flag is set.
pub struct Connection {
    pub state: Arc<DepthState>,
    stop: Arc<AtomicBool>,
    stream: Mutex<Option<UnixStream>>,
    subscribers: SubscriberList,
}

impl Connection {
    /// Connect to the socket path and start the reader thread.
    pub fn connect(path: &PathBuf) -> Result<Self, SocketError> {
        let stream = UnixStream::connect(path).map_err(SocketError::Connect)?;
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .ok();
        stream.set_nonblocking(false).ok();

        let state = Arc::new(DepthState::default());
        // Mark connected before returning so callers racing the reader
        // thread's first wakeup see the right state immediately.
        state.set_connected(true);
        let stop = Arc::new(AtomicBool::new(false));
        let conn = Self {
            state: Arc::clone(&state),
            stop: Arc::clone(&stop),
            stream: Mutex::new(Some(stream.try_clone().map_err(SocketError::Connect)?)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        };

        let mut reader = stream;
        let st = Arc::clone(&state);
        let sp = Arc::clone(&stop);
        let subs = Arc::clone(&conn.subscribers);
        std::thread::Builder::new()
            .name("fun60-uds-reader".into())
            .spawn(move || reader_loop(&mut reader, &st, &sp, &subs))
            .expect("spawn reader thread");

        Ok(conn)
    }

    /// Subscribe to depth updates: returns a bounded drop-oldest queue fed by
    /// the reader thread. The current snapshot is queued first, then live
    /// events; a `DepthUpdate::Disconnected` update is queued when the
    /// connection goes down (followed by nothing — the receiver's
    /// `recv_timeout` returns None only after closure).
    pub fn subscribe(&self) -> EventReceiver {
        self.subscribe_with_capacity(DEFAULT_QUEUE_CAPACITY)
    }

    /// [`Connection::subscribe`] with an explicit queue capacity.
    pub fn subscribe_with_capacity(&self, capacity: usize) -> EventReceiver {
        // Seed the queue with the current state as a snapshot so the
        // subscriber immediately knows the last-known depths.
        let snapshot = self
            .state
            .snapshot_all()
            .into_iter()
            .map(|(key_index, depth_raw)| DepthEvent {
                key_index,
                depth_raw,
            })
            .collect();
        let ring = EventRing {
            deque: std::collections::VecDeque::from(vec![DepthUpdate::Snapshot(snapshot)]),
            capacity,
            closed: false,
        };
        let arc: RingShared = Arc::new((Mutex::new(ring), std::sync::Condvar::new()));
        self.subscribers.lock().unwrap().push(Arc::clone(&arc));
        EventReceiver { inner: arc }
    }

    /// Send a query command; the next snapshot frame updates the cache.
    pub fn query(&self) -> Result<(), SocketError> {
        self.send_cmd(b'Q')
    }

    fn send_cmd(&self, cmd: u8) -> Result<(), SocketError> {
        let mut guard = self.stream.lock().unwrap();
        if let Some(s) = guard.as_mut() {
            s.write_all(&[cmd]).map_err(SocketError::Disconnected)?;
            Ok(())
        } else {
            Err(SocketError::Disconnected(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "no stream",
            )))
        }
    }

    /// Signal the reader thread to stop (idempotent).
    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.send_cmd(b'X');
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn reader_loop(
    stream: &mut UnixStream,
    state: &DepthState,
    stop: &AtomicBool,
    subscribers: &SubscriberList,
) {
    let mut buf = [0u8; 4096];
    state.set_connected(true);
    while !stop.load(Ordering::Relaxed) {
        match stream.read(&mut buf) {
            Ok(0) => break, // EOF: server closed
            Ok(n) => {
                match parse_frames(&buf[..n]) {
                    Ok(frames) => {
                        for frame in frames {
                            let update = match frame {
                                Frame::Snapshot(entries) => {
                                    // Snapshot replaces state: keys not in it
                                    // are at rest.
                                    state.clear();
                                    for e in &entries {
                                        state.apply(e.key_index, e.depth_raw);
                                    }
                                    Some(DepthUpdate::Snapshot(entries))
                                }
                                Frame::Event(e) => {
                                    state.apply(e.key_index, e.depth_raw);
                                    Some(DepthUpdate::Event(e))
                                }
                                Frame::Error(code) => {
                                    eprintln!("fun60 plugin: server error frame {code}");
                                    None
                                }
                            };
                            if let Some(update) = update {
                                fanout(subscribers, update);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("fun60 plugin: {e}");
                        break;
                    }
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // read timeout: loop to re-check stop flag
            }
            Err(_) => break,
        }
    }
    // Connection is down: notify subscribers (Disconnected update, then close).
    state.set_connected(false);
    let mut subs = subscribers.lock().unwrap().clone();
    for sub in &subs {
        let (lock, cvar) = &**sub;
        let mut ring = lock.lock().unwrap();
        push_oldest_drop(&mut ring, DepthUpdate::Disconnected);
        ring.closed = true;
        cvar.notify_all();
    }
    // All subscribers are closed now; drop them.
    subs.clear();
    subscribers.lock().unwrap().clear();
}

/// Push an update to every subscriber queue (drop-oldest on overflow, dead
/// subscribers' rings are lazily removed).
fn fanout(subscribers: &SubscriberList, update: DepthUpdate) {
    let mut subs = subscribers.lock().unwrap();
    subs.retain(|sub| Arc::strong_count(sub) > 1); // drop receivers nobody holds
    for sub in subs.iter() {
        let (lock, cvar) = &**sub;
        let mut ring = lock.lock().unwrap();
        push_oldest_drop(&mut ring, update.clone());
        cvar.notify_one();
    }
}

/// Push into the ring, dropping the OLDEST entry on overflow.
///
/// Drop-oldest is deliberate: each DepthEvent carries the absolute depth, so
/// dropping an old event loses nothing about the *current* position, and
/// keeping the newest preserves release edges (the last event for a key may
/// be its return-to-rest report).
fn push_oldest_drop(ring: &mut EventRing, update: DepthUpdate) {
    if ring.capacity > 0 {
        while ring.deque.len() >= ring.capacity {
            ring.deque.pop_front();
        }
    }
    ring.deque.push_back(update);
}

#[cfg(test)]
mod tests {
use super::*;

    #[test]
    fn parses_event_frame() {
        let buf = [b'D', 10, 0x1F, 0x00]; // key 10, depth 31
        assert_eq!(
            parse_frames(&buf).unwrap(),
            vec![Frame::Event(DepthEvent {
                key_index: 10,
                depth_raw: 31
            })]
        );
    }

    #[test]
    fn parses_snapshot_frame() {
        let buf = [b'S', 2, 1, 0x64, 0x00, 2, 0xC8, 0x01];
        assert_eq!(
            parse_frames(&buf).unwrap(),
            vec![Frame::Snapshot(vec![
                DepthEvent {
                    key_index: 1,
                    depth_raw: 100
                },
                DepthEvent {
                    key_index: 2,
                    depth_raw: 456
                },
            ])]
        );
    }

    #[test]
    fn parses_back_to_back_frames() {
        let buf = [b'D', 5, 0x0A, 0x00, b'D', 6, 0x14, 0x00, b'E', 1];
        assert_eq!(
            parse_frames(&buf).unwrap(),
            vec![
                Frame::Event(DepthEvent {
                    key_index: 5,
                    depth_raw: 10
                }),
                Frame::Event(DepthEvent {
                    key_index: 6,
                    depth_raw: 20
                }),
                Frame::Error(1),
            ]
        );
    }

    #[test]
    fn truncated_frame_is_error() {
        assert!(parse_frames(&[b'D', 5]).is_err());
        assert!(parse_frames(&[b'S', 3, 1, 0, 0]).is_err());
    }

    #[test]
    fn unknown_frame_is_error() {
        assert!(parse_frames(&[b'Z', 0, 0, 0]).is_err());
    }

    #[test]
    fn depth_state_applies_and_snapshots() {
        let st = DepthState::default();
        assert_eq!(st.get(9), 0);
        st.apply(9, 200);
        st.apply(10, 0); // at-rest report
        assert_eq!(st.get(9), 200);
        assert_eq!(st.get(10), 0);
        assert_eq!(st.pressed(), vec![(9, 200)]);
    }

    // --- event queue (drop-oldest ring) tests ---

    /// Build a receiver-style ring directly (no live connection needed).
    fn test_ring(capacity: usize) -> Arc<(Mutex<EventRing>, std::sync::Condvar)> {
        Arc::new((
            Mutex::new(EventRing {
                deque: std::collections::VecDeque::new(),
                capacity,
                closed: false,
            }),
            std::sync::Condvar::new(),
        ))
    }

    fn recv_all(rx: &EventReceiver) -> Vec<DepthUpdate> {
        let mut out = Vec::new();
        while let Some(ev) = rx.try_recv() {
            out.push(ev);
        }
        out
    }

    #[test]
    fn drop_oldest_on_overflow() {
        let arc = test_ring(3);
        let rx = EventReceiver {
            inner: Arc::clone(&arc),
        };
        {
            let (lock, _) = &*arc;
            let mut ring = lock.lock().unwrap();
            for i in 0..5u16 {
                push_oldest_drop(
                    &mut ring,
                    DepthUpdate::Event(DepthEvent {
                        key_index: 1,
                        depth_raw: i,
                    }),
                );
            }
        }
        let got = recv_all(&rx);
        assert_eq!(got.len(), 3, "capacity respected");
        // Oldest two (0, 1) dropped; newest three kept.
        assert_eq!(
            got,
            vec![
                DepthUpdate::Event(DepthEvent {
                    key_index: 1,
                    depth_raw: 2
                }),
                DepthUpdate::Event(DepthEvent {
                    key_index: 1,
                    depth_raw: 3
                }),
                DepthUpdate::Event(DepthEvent {
                    key_index: 1,
                    depth_raw: 4
                }),
            ]
        );
    }

    #[test]
    fn recv_timeout_returns_none_when_empty() {
        let arc = test_ring(4);
        let rx = EventReceiver {
            inner: Arc::clone(&arc),
        };
        assert!(rx.recv_timeout(Duration::from_millis(20)).is_none());
    }

    #[test]
    fn recv_wakes_on_push() {
        let arc = test_ring(4);
        let rx = EventReceiver {
            inner: Arc::clone(&arc),
        };
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            let (lock, cvar) = &*arc;
            let mut ring = lock.lock().unwrap();
            push_oldest_drop(
                &mut ring,
                DepthUpdate::Event(DepthEvent {
                    key_index: 3,
                    depth_raw: 77,
                }),
            );
            cvar.notify_one();
        });
        let got = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("should wake and receive");
        assert_eq!(
            got,
            DepthUpdate::Event(DepthEvent {
                key_index: 3,
                depth_raw: 77
            })
        );
    }

    #[test]
    fn closed_ring_drains_then_returns_none() {
        let arc = test_ring(4);
        let rx = EventReceiver {
            inner: Arc::clone(&arc),
        };
        {
            let (lock, cvar) = &*arc;
            let mut ring = lock.lock().unwrap();
            push_oldest_drop(
                &mut ring,
                DepthUpdate::Event(DepthEvent {
                    key_index: 1,
                    depth_raw: 5,
                }),
            );
            ring.closed = true;
            cvar.notify_all();
        }
        // Drains remaining, then None.
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_some());
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_none());
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_none());
    }

    #[test]
    fn disconnected_update_then_close() {
        let arc = test_ring(4);
        let rx = EventReceiver {
            inner: Arc::clone(&arc),
        };
        {
            let (lock, cvar) = &*arc;
            let mut ring = lock.lock().unwrap();
            push_oldest_drop(&mut ring, DepthUpdate::Disconnected);
            ring.closed = true;
            cvar.notify_all();
        }
        assert_eq!(
            rx.recv_timeout(Duration::from_millis(50)),
            Some(DepthUpdate::Disconnected)
        );
        assert!(rx.recv_timeout(Duration::from_millis(50)).is_none());
    }

    #[test]
    fn zero_capacity_is_ignored() {
        // capacity 0 must not drop everything: guarded by `if capacity > 0`.
        let arc = test_ring(0);
        let rx = EventReceiver {
            inner: Arc::clone(&arc),
        };
        {
            let (lock, _) = &*arc;
            let mut ring = lock.lock().unwrap();
            push_oldest_drop(
                &mut ring,
                DepthUpdate::Event(DepthEvent {
                    key_index: 1,
                    depth_raw: 9,
                }),
            );
        }
        assert_eq!(rx.len(), 1, "capacity 0 means unbounded");
    }

    #[test]
    fn event_iterator() {
        use std::sync::TryLockError;
        const ITER_COUNT: usize = 10;
        let arc = test_ring(ITER_COUNT);
        let rx = EventReceiver {
            inner: Arc::clone(&arc)
        };
        let handle = std::thread::spawn(move || {
            rx.into_iter()
              .count()
        });
        {
            let (lock, cvar) = &*arc;
            let mut ring = loop {
                let mut cnt = 0;
                match lock.try_lock() {
                    Ok(guard) => {
                        break guard;
                    },
                    Err(TryLockError::WouldBlock) => {
                        std::thread::sleep(Duration::from_secs(1));
                        cnt += 1;
                    },
                    Err(e) => panic!("{e}")
                }
                if cnt >= 5 {
                    panic!("timeout!");
                }
            };
            let ring = &mut *ring;
            for i in 0..ITER_COUNT {
                push_oldest_drop(
                    ring,
                    DepthUpdate::Event(DepthEvent {
                        key_index: 1,
                        depth_raw: i as u16 * 10,
                    }),
                );
            }
            cvar.notify_all();
            ring.closed = true;
        }

        let res = handle.join().expect("failed to join the thread");
        assert_eq!(ITER_COUNT, res)
    }
}
