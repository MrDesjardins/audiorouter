//! UDP audio streaming for the Network Send and Network Receive nodes.
//!
//! A Network Send node on one computer streams its input to the address of
//! a Network Receive node on another computer on the same network (for
//! example a gaming PC feeding a streaming PC). Each datagram carries one
//! quantum of interleaved float32 audio at the internal 48 kHz rate:
//!
//! ```text
//! offset  size  field
//!      0     4  magic "ARNA"
//!      4     1  version (1)
//!      5     1  channels (1..=2)
//!      6     2  frames per packet, little endian (1..=MAX_NETWORK_PACKET_FRAMES)
//!      8     4  sample rate, little endian (48000)
//!     12     4  stream id, little endian (random per sender start)
//!     16     4  sequence, little endian (wraps)
//!     20     .  frames * channels float32, little endian, interleaved
//! ```
//!
//! Realtime boundary: the graph tap only copies a quantum into a
//! preallocated packet pool; a separate thread performs socket I/O. The
//! receive thread validates every datagram (magic, sizes, rate, and the
//! configured sender address) before it reaches a bounded queue, and the
//! audio side plays that queue out at the wall-clock rate through a jitter
//! buffer with loss concealment and gentle drift correction. No datagram is
//! ever interpreted as anything but audio, and no address other than the
//! configured one is sent to or accepted from.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_queue::ArrayQueue;

use crate::{AudioCaptureSource, AudioError, CapturePacket};

pub const NETWORK_AUDIO_MAGIC: [u8; 4] = *b"ARNA";
pub const NETWORK_AUDIO_VERSION: u8 = 1;
pub const NETWORK_AUDIO_HEADER_BYTES: usize = 20;
/// Largest quantum accepted in one datagram. 256 stereo frames is 2,068
/// bytes; the normal 128-frame quantum (1,044 bytes) fits one Ethernet frame.
pub const MAX_NETWORK_PACKET_FRAMES: usize = 256;
pub const MAX_NETWORK_CHANNELS: usize = 2;
pub const MAX_NETWORK_PACKET_BYTES: usize =
    NETWORK_AUDIO_HEADER_BYTES + MAX_NETWORK_PACKET_FRAMES * MAX_NETWORK_CHANNELS * 4;
/// Packets waiting for the sender thread (about 170 ms of 128-frame quanta).
const SEND_QUEUE_PACKETS: usize = 64;
/// Received audio kept at most (about 1.4 s of 128-frame quanta).
const RECEIVE_QUEUE_PACKETS: usize = 512;
/// Lost packets concealed with silence; a larger gap is a resynchronisation.
const MAX_CONCEALED_PACKETS: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetworkPacketHeader {
    pub channels: u8,
    pub frames: u16,
    pub sample_rate_hz: u32,
    pub stream_id: u32,
    pub sequence: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum NetworkAudioError {
    /// Not an AudioRouter audio datagram, or a malformed one.
    Malformed,
    /// A well-formed datagram in a format this receiver does not play.
    Unsupported,
    BufferTooSmall,
}

/// Encode one packet into `out` and return its length.
pub fn encode_network_packet(
    header: NetworkPacketHeader,
    interleaved: &[f32],
    out: &mut [u8],
) -> Result<usize, NetworkAudioError> {
    let channels = usize::from(header.channels);
    let frames = usize::from(header.frames);
    if !(1..=MAX_NETWORK_CHANNELS).contains(&channels)
        || !(1..=MAX_NETWORK_PACKET_FRAMES).contains(&frames)
        || interleaved.len() != channels * frames
    {
        return Err(NetworkAudioError::Unsupported);
    }
    let length = NETWORK_AUDIO_HEADER_BYTES + interleaved.len() * 4;
    if out.len() < length {
        return Err(NetworkAudioError::BufferTooSmall);
    }
    out[0..4].copy_from_slice(&NETWORK_AUDIO_MAGIC);
    out[4] = NETWORK_AUDIO_VERSION;
    out[5] = header.channels;
    out[6..8].copy_from_slice(&header.frames.to_le_bytes());
    out[8..12].copy_from_slice(&header.sample_rate_hz.to_le_bytes());
    out[12..16].copy_from_slice(&header.stream_id.to_le_bytes());
    out[16..20].copy_from_slice(&header.sequence.to_le_bytes());
    for (sample, bytes) in interleaved
        .iter()
        .zip(out[NETWORK_AUDIO_HEADER_BYTES..length].chunks_exact_mut(4))
    {
        bytes.copy_from_slice(&sample.to_le_bytes());
    }
    Ok(length)
}

/// Validate a datagram and return its header and sample payload. Only the
/// internal 48 kHz rate is accepted; the payload length must match exactly.
pub fn decode_network_packet(
    datagram: &[u8],
) -> Result<(NetworkPacketHeader, &[u8]), NetworkAudioError> {
    if datagram.len() < NETWORK_AUDIO_HEADER_BYTES || datagram[0..4] != NETWORK_AUDIO_MAGIC {
        return Err(NetworkAudioError::Malformed);
    }
    if datagram[4] != NETWORK_AUDIO_VERSION {
        return Err(NetworkAudioError::Unsupported);
    }
    let header = NetworkPacketHeader {
        channels: datagram[5],
        frames: u16::from_le_bytes([datagram[6], datagram[7]]),
        sample_rate_hz: u32::from_le_bytes(datagram[8..12].try_into().expect("4 bytes")),
        stream_id: u32::from_le_bytes(datagram[12..16].try_into().expect("4 bytes")),
        sequence: u32::from_le_bytes(datagram[16..20].try_into().expect("4 bytes")),
    };
    let channels = usize::from(header.channels);
    let frames = usize::from(header.frames);
    if !(1..=MAX_NETWORK_CHANNELS).contains(&channels)
        || !(1..=MAX_NETWORK_PACKET_FRAMES).contains(&frames)
        || header.sample_rate_hz != audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ
    {
        return Err(NetworkAudioError::Unsupported);
    }
    let payload = &datagram[NETWORK_AUDIO_HEADER_BYTES..];
    if payload.len() != channels * frames * 4 {
        return Err(NetworkAudioError::Malformed);
    }
    Ok((header, payload))
}

/// Parse a node's IP literal and port into a socket address.
pub fn network_socket_address(address: &str, port: u16) -> Option<SocketAddr> {
    if port == 0 || !audiorouter_domain::valid_network_address(address) {
        return None;
    }
    address
        .parse::<IpAddr>()
        .ok()
        .map(|ip| SocketAddr::new(ip, port))
}

fn stream_id() -> u32 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos()),
    );
    hasher.write_u32(std::process::id());
    hasher.finish() as u32
}

/// Counters of one Network Send node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetworkSendStats {
    pub sent_packets: u64,
    /// Quanta not sent because the send queue was full.
    pub dropped_packets: u64,
    pub send_errors: u64,
    /// The Windows socket error of the latest failed send (for example
    /// 10054: the receiving computer answered "port closed").
    pub last_error_code: Option<i32>,
    /// The local address the audio leaves from; the receiver must expect
    /// this address (a computer with several adapters may use another one).
    pub local_address: Option<SocketAddr>,
}

struct SendPacket {
    bytes: Box<[u8]>,
    length: usize,
}

struct SenderShared {
    free: ArrayQueue<SendPacket>,
    ready: ArrayQueue<SendPacket>,
    running: AtomicBool,
    stream_id: u32,
    sequence: AtomicU32,
    sent: AtomicU64,
    dropped: AtomicU64,
    errors: AtomicU64,
    /// Latest send error's OS code; 0 when none.
    last_error: std::sync::atomic::AtomicI32,
    /// The socket's local address, refreshed when it reconnects.
    local: Mutex<Option<SocketAddr>>,
    thread: std::sync::OnceLock<std::thread::Thread>,
    /// A new destination requested while running; applied by the I/O thread
    /// before its next send (a new socket when the IP family changes).
    retarget: Mutex<Option<SocketAddr>>,
}

impl SenderShared {
    fn record_error(&self, error: &std::io::Error) {
        self.errors.fetch_add(1, Ordering::Relaxed);
        self.last_error
            .store(error.raw_os_error().unwrap_or(-1), Ordering::Relaxed);
    }

    fn record_local(&self, socket: &UdpSocket) {
        if let Ok(mut local) = self.local.lock() {
            *local = socket.local_addr().ok();
        }
    }
}

/// The local address this computer uses to reach `remote` (route lookup by a
/// UDP connect; no packet is sent). On the receiving computer this is the
/// address the sender must target; it reveals a wrong adapter or subnet.
pub fn local_address_toward(remote: IpAddr) -> Option<IpAddr> {
    let probe = connected_socket(SocketAddr::new(remote, 9)).ok()?;
    probe.local_addr().ok().map(|local| local.ip())
}

fn connected_socket(destination: SocketAddr) -> Result<UdpSocket, std::io::Error> {
    let bind: SocketAddr = if destination.is_ipv4() {
        (std::net::Ipv4Addr::UNSPECIFIED, 0).into()
    } else {
        (std::net::Ipv6Addr::UNSPECIFIED, 0).into()
    };
    let socket = UdpSocket::bind(bind)?;
    socket.connect(destination)?;
    Ok(socket)
}

/// Streams processed quanta to one address. The graph side is the
/// [`NetworkSendTap`]; dropping the sender stops and joins its I/O thread.
pub struct NetworkSender {
    shared: Arc<SenderShared>,
    destination: Mutex<SocketAddr>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl NetworkSender {
    pub fn start(destination: SocketAddr) -> Result<Self, AudioError> {
        let mut socket = connected_socket(destination).map_err(network_error)?;
        let shared = Arc::new(SenderShared {
            free: ArrayQueue::new(SEND_QUEUE_PACKETS),
            ready: ArrayQueue::new(SEND_QUEUE_PACKETS),
            running: AtomicBool::new(true),
            stream_id: stream_id(),
            sequence: AtomicU32::new(0),
            sent: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            last_error: std::sync::atomic::AtomicI32::new(0),
            local: Mutex::new(socket.local_addr().ok()),
            thread: std::sync::OnceLock::new(),
            retarget: Mutex::new(None),
        });
        for _ in 0..SEND_QUEUE_PACKETS {
            let _ = shared.free.push(SendPacket {
                bytes: vec![0; MAX_NETWORK_PACKET_BYTES].into_boxed_slice(),
                length: 0,
            });
        }
        let thread_shared = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("audiorouter-network-send".into())
            .spawn(move || {
                while thread_shared.running.load(Ordering::Acquire) {
                    let retarget = thread_shared
                        .retarget
                        .lock()
                        .ok()
                        .and_then(|mut next| next.take());
                    if let Some(next) = retarget {
                        let reconnected = if socket
                            .local_addr()
                            .is_ok_and(|local| local.is_ipv4() == next.is_ipv4())
                        {
                            socket.connect(next).map(|()| None)
                        } else {
                            connected_socket(next).map(Some)
                        };
                        match reconnected {
                            Ok(Some(replacement)) => socket = replacement,
                            Ok(None) => {}
                            Err(error) => thread_shared.record_error(&error),
                        }
                        thread_shared.record_local(&socket);
                    }
                    while let Some(packet) = thread_shared.ready.pop() {
                        match socket.send(&packet.bytes[..packet.length]) {
                            Ok(_) => {
                                thread_shared.sent.fetch_add(1, Ordering::Relaxed);
                            }
                            Err(error) => thread_shared.record_error(&error),
                        }
                        let _ = thread_shared.free.push(packet);
                    }
                    std::thread::park_timeout(Duration::from_millis(20));
                }
            })
            .map_err(network_error)?;
        let _ = shared.thread.set(handle.thread().clone());
        Ok(Self {
            shared,
            destination: Mutex::new(destination),
            handle: Some(handle),
        })
    }

    pub fn destination(&self) -> SocketAddr {
        self.destination.lock().map_or_else(
            |poisoned| *poisoned.into_inner(),
            |destination| *destination,
        )
    }

    /// Send to another address from now on, without interrupting the
    /// stream: a live address or port change of a playing Network Send.
    pub fn retarget(&self, destination: SocketAddr) {
        if let Ok(mut current) = self.destination.lock() {
            *current = destination;
        }
        if let Ok(mut next) = self.shared.retarget.lock() {
            *next = Some(destination);
        }
        if let Some(thread) = self.shared.thread.get() {
            thread.unpark();
        }
    }

    /// The realtime graph tap feeding this sender.
    pub fn tap(&self) -> NetworkSendTap {
        NetworkSendTap {
            shared: Arc::clone(&self.shared),
        }
    }

    pub fn stats(&self) -> NetworkSendStats {
        NetworkSendStats {
            sent_packets: self.shared.sent.load(Ordering::Relaxed),
            dropped_packets: self.shared.dropped.load(Ordering::Relaxed),
            send_errors: self.shared.errors.load(Ordering::Relaxed),
            last_error_code: Some(self.shared.last_error.load(Ordering::Relaxed))
                .filter(|code| *code != 0),
            local_address: self.shared.local.lock().ok().and_then(|local| *local),
        }
    }
}

impl Drop for NetworkSender {
    fn drop(&mut self) {
        self.shared.running.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}

/// Realtime side of a [`NetworkSender`]: copies each quantum into a
/// preallocated packet and wakes the I/O thread. Never blocks or allocates;
/// when the queue is full the quantum is counted as dropped.
pub struct NetworkSendTap {
    shared: Arc<SenderShared>,
}

impl audiorouter_engine::AudioTap for NetworkSendTap {
    fn on_processed_block(&self, _start_frame: u64, block: &audiorouter_engine::AudioBlock) {
        let channels = block.channels();
        let frames = block.frames();
        if !self.shared.running.load(Ordering::Acquire)
            || !(1..=MAX_NETWORK_CHANNELS).contains(&channels)
            || !(1..=MAX_NETWORK_PACKET_FRAMES).contains(&frames)
        {
            return;
        }
        let Some(mut packet) = self.shared.free.pop() else {
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        let header = NetworkPacketHeader {
            channels: channels as u8,
            frames: frames as u16,
            sample_rate_hz: audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ,
            stream_id: self.shared.stream_id,
            sequence: self.shared.sequence.fetch_add(1, Ordering::Relaxed),
        };
        let bytes = &mut packet.bytes;
        bytes[0..4].copy_from_slice(&NETWORK_AUDIO_MAGIC);
        bytes[4] = NETWORK_AUDIO_VERSION;
        bytes[5] = header.channels;
        bytes[6..8].copy_from_slice(&header.frames.to_le_bytes());
        bytes[8..12].copy_from_slice(&header.sample_rate_hz.to_le_bytes());
        bytes[12..16].copy_from_slice(&header.stream_id.to_le_bytes());
        bytes[16..20].copy_from_slice(&header.sequence.to_le_bytes());
        let mut offset = NETWORK_AUDIO_HEADER_BYTES;
        for frame in 0..frames {
            for channel in 0..channels {
                let sample = block.channel(channel).map_or(0.0, |samples| samples[frame]);
                let sample = if sample.is_finite() { sample } else { 0.0 };
                bytes[offset..offset + 4].copy_from_slice(&sample.to_le_bytes());
                offset += 4;
            }
        }
        packet.length = offset;
        if let Err(packet) = self.shared.ready.push(packet) {
            let _ = self.shared.free.push(packet);
            self.shared.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        }
        if let Some(thread) = self.shared.thread.get() {
            thread.unpark();
        }
    }
}

/// Counters of one Network Receive node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NetworkReceiveStats {
    /// The most recent address that sent AudioRouter audio here but is not
    /// the configured sender (for example another network adapter of the
    /// sending computer), so the user can correct the address.
    pub last_rejected_sender: Option<IpAddr>,
    pub received_packets: u64,
    /// Sequence gaps concealed with silence.
    pub lost_packets: u64,
    /// Duplicate or out-of-order packets that arrived too late to play.
    pub late_packets: u64,
    /// Datagrams from another address or not in the audio format.
    pub rejected_datagrams: u64,
    /// The playout buffer ran empty (an audible gap).
    pub underruns: u64,
    /// Packets discarded because the queue was full or far too deep.
    pub overflow_packets: u64,
    /// Audio currently buffered ahead of playout, in frames.
    pub buffered_frames: u64,
    /// Socket receive failures other than the periodic read timeout.
    pub receive_errors: u64,
    /// The Windows socket error of the latest receive failure.
    pub last_error_code: Option<i32>,
    /// This computer's address toward the configured sender: the address
    /// the sending computer must target.
    pub local_address_toward_sender: Option<IpAddr>,
}

struct ReceivePacket {
    samples: Box<[f32]>,
    channels: usize,
    frames: usize,
}

struct ReceiverShared {
    free: ArrayQueue<ReceivePacket>,
    ready: ArrayQueue<ReceivePacket>,
    queued_frames: AtomicUsize,
    running: AtomicBool,
    received: AtomicU64,
    lost: AtomicU64,
    late: AtomicU64,
    rejected: AtomicU64,
    underruns: AtomicU64,
    overflows: AtomicU64,
    receive_errors: AtomicU64,
    /// Latest receive error OS code; 0 when none.
    last_error: std::sync::atomic::AtomicI32,
    /// Route lookup toward the sender, refreshed when the sender changes.
    local_toward_sender: Mutex<Option<IpAddr>>,
    /// Written by the receive thread only; read for telemetry.
    last_rejected_sender: Mutex<Option<IpAddr>>,
    /// The only address accepted; changeable while running.
    sender: Mutex<IpAddr>,
    /// Jitter-buffer target in frames; changeable while running.
    target_frames: AtomicUsize,
}

impl ReceiverShared {
    fn push(&self, packet: ReceivePacket) {
        let frames = packet.frames;
        match self.ready.push(packet) {
            Ok(()) => {
                self.queued_frames.fetch_add(frames, Ordering::AcqRel);
            }
            Err(packet) => {
                let _ = self.free.push(packet);
                self.overflows.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

/// Playout state, touched only by the audio service thread.
struct Playout {
    current: Option<ReceivePacket>,
    offset: usize,
    playing: bool,
    started: Option<Instant>,
    delivered_frames: u64,
    scratch: Box<[f32]>,
}

/// Receives one sender's stream and plays it out as a paced capture source.
pub struct NetworkReceiver {
    shared: Arc<ReceiverShared>,
    playout: Mutex<Playout>,
    listen: SocketAddr,
    handle: Option<std::thread::JoinHandle<()>>,
}

fn same_host(left: IpAddr, right: IpAddr) -> bool {
    let canonical = |ip: IpAddr| match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
        ip => ip,
    };
    canonical(left) == canonical(right)
}

impl NetworkReceiver {
    /// Listen on `port` for audio from `sender` only, buffering `buffer_ms`
    /// ahead of playout.
    pub fn start(sender: IpAddr, port: u16, buffer_ms: f64) -> Result<Self, AudioError> {
        if port == 0 || !buffer_ms.is_finite() {
            return Err(AudioError::InvalidFrameSize);
        }
        let listen: SocketAddr = if sender.is_ipv4() {
            (std::net::Ipv4Addr::UNSPECIFIED, port).into()
        } else {
            (std::net::Ipv6Addr::UNSPECIFIED, port).into()
        };
        let socket = UdpSocket::bind(listen).map_err(network_error)?;
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(network_error)?;
        let rate = f64::from(audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ);
        let target_frames = buffer_target_frames(buffer_ms, rate);
        let shared = Arc::new(ReceiverShared {
            free: ArrayQueue::new(RECEIVE_QUEUE_PACKETS),
            ready: ArrayQueue::new(RECEIVE_QUEUE_PACKETS),
            queued_frames: AtomicUsize::new(0),
            running: AtomicBool::new(true),
            received: AtomicU64::new(0),
            lost: AtomicU64::new(0),
            late: AtomicU64::new(0),
            rejected: AtomicU64::new(0),
            underruns: AtomicU64::new(0),
            overflows: AtomicU64::new(0),
            receive_errors: AtomicU64::new(0),
            last_error: std::sync::atomic::AtomicI32::new(0),
            local_toward_sender: Mutex::new(local_address_toward(sender)),
            last_rejected_sender: Mutex::new(None),
            sender: Mutex::new(sender),
            target_frames: AtomicUsize::new(target_frames),
        });
        for _ in 0..RECEIVE_QUEUE_PACKETS {
            let _ = shared.free.push(ReceivePacket {
                samples: vec![0.0; MAX_NETWORK_PACKET_FRAMES * MAX_NETWORK_CHANNELS]
                    .into_boxed_slice(),
                channels: 0,
                frames: 0,
            });
        }
        let thread_shared = Arc::clone(&shared);
        let handle = std::thread::Builder::new()
            .name("audiorouter-network-receive".into())
            .spawn(move || receive_loop(&socket, &thread_shared))
            .map_err(network_error)?;
        Ok(Self {
            shared,
            playout: Mutex::new(Playout {
                current: None,
                offset: 0,
                playing: false,
                started: None,
                delivered_frames: 0,
                scratch: vec![0.0; (MAX_NETWORK_PACKET_FRAMES + 1) * MAX_NETWORK_CHANNELS]
                    .into_boxed_slice(),
            }),
            listen,
            handle: Some(handle),
        })
    }

    pub fn listen_address(&self) -> SocketAddr {
        self.listen
    }

    /// Accept audio from another sending computer and/or use another buffer,
    /// while playing (a live edit). The port stays bound; a port change
    /// needs a new receiver. Returns false when the IP family differs from
    /// the bound socket, which also needs a new receiver.
    pub fn reconfigure(&self, sender: IpAddr, buffer_ms: f64) -> bool {
        if sender.is_ipv4() != self.listen.is_ipv4() || !buffer_ms.is_finite() {
            return false;
        }
        if let Ok(mut current) = self.shared.sender.lock() {
            *current = sender;
        }
        if let Ok(mut local) = self.shared.local_toward_sender.lock() {
            *local = local_address_toward(sender);
        }
        if let Ok(mut last) = self.shared.last_rejected_sender.lock() {
            *last = None;
        }
        let rate = f64::from(audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ);
        self.shared
            .target_frames
            .store(buffer_target_frames(buffer_ms, rate), Ordering::Release);
        true
    }

    pub fn stats(&self) -> NetworkReceiveStats {
        NetworkReceiveStats {
            last_rejected_sender: self
                .shared
                .last_rejected_sender
                .lock()
                .ok()
                .and_then(|sender| *sender),
            received_packets: self.shared.received.load(Ordering::Relaxed),
            lost_packets: self.shared.lost.load(Ordering::Relaxed),
            late_packets: self.shared.late.load(Ordering::Relaxed),
            rejected_datagrams: self.shared.rejected.load(Ordering::Relaxed),
            underruns: self.shared.underruns.load(Ordering::Relaxed),
            overflow_packets: self.shared.overflows.load(Ordering::Relaxed),
            buffered_frames: self.shared.queued_frames.load(Ordering::Acquire) as u64,
            receive_errors: self.shared.receive_errors.load(Ordering::Relaxed),
            last_error_code: Some(self.shared.last_error.load(Ordering::Relaxed))
                .filter(|code| *code != 0),
            local_address_toward_sender: self
                .shared
                .local_toward_sender
                .lock()
                .ok()
                .and_then(|local| *local),
        }
    }

    /// Restart playout pacing, for example when the session starts again.
    pub fn reset_playout(&self) {
        if let Ok(mut playout) = self.playout.lock() {
            playout.started = None;
            playout.delivered_frames = 0;
            playout.playing = false;
        }
    }
}

impl Drop for NetworkReceiver {
    fn drop(&mut self) {
        self.shared.running.store(false, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            // The socket read times out within 100 ms, then the loop exits.
            let _ = handle.join();
        }
    }
}

fn buffer_target_frames(buffer_ms: f64, rate: f64) -> usize {
    (buffer_ms.clamp(
        audiorouter_domain::MIN_NETWORK_BUFFER_MS,
        audiorouter_domain::MAX_NETWORK_BUFFER_MS,
    ) * rate
        / 1_000.0) as usize
}

fn receive_loop(socket: &UdpSocket, shared: &ReceiverShared) {
    let mut datagram = vec![0_u8; MAX_NETWORK_PACKET_BYTES + 1];
    let mut stream: Option<(u32, u32)> = None; // (stream id, next sequence)
    while shared.running.load(Ordering::Acquire) {
        let (length, from) = match socket.recv_from(&mut datagram) {
            Ok(received) => received,
            // The 100 ms read timeout only re-checks `running`.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            Err(error) => {
                shared.receive_errors.fetch_add(1, Ordering::Relaxed);
                shared
                    .last_error
                    .store(error.raw_os_error().unwrap_or(-1), Ordering::Relaxed);
                continue;
            }
        };
        if length > MAX_NETWORK_PACKET_BYTES {
            shared.rejected.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        let sender = shared
            .sender
            .lock()
            .map_or_else(|poisoned| *poisoned.into_inner(), |sender| *sender);
        if !same_host(from.ip(), sender) {
            shared.rejected.fetch_add(1, Ordering::Relaxed);
            // Remember only real AudioRouter audio, never arbitrary traffic,
            // so the hint names the sending computer's actual address.
            if decode_network_packet(&datagram[..length]).is_ok() {
                if let Ok(mut last) = shared.last_rejected_sender.lock() {
                    *last = Some(match from.ip() {
                        IpAddr::V6(v6) => v6.to_ipv4_mapped().map_or(IpAddr::V6(v6), IpAddr::V4),
                        ip => ip,
                    });
                }
            }
            continue;
        }
        let Ok((header, payload)) = decode_network_packet(&datagram[..length]) else {
            shared.rejected.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        let channels = usize::from(header.channels);
        let frames = usize::from(header.frames);
        match stream {
            Some((id, next)) if id == header.stream_id => {
                let ahead = header.sequence.wrapping_sub(next);
                if ahead >= u32::MAX / 2 {
                    // Behind the playout sequence: duplicate or reordered.
                    shared.late.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                if ahead > 0 && ahead <= MAX_CONCEALED_PACKETS {
                    for _ in 0..ahead {
                        if let Some(mut silence) = shared.free.pop() {
                            silence.samples[..channels * frames].fill(0.0);
                            silence.channels = channels;
                            silence.frames = frames;
                            shared.push(silence);
                        }
                    }
                    shared.lost.fetch_add(u64::from(ahead), Ordering::Relaxed);
                }
            }
            // A new stream id is a restarted sender: follow it.
            _ => {}
        }
        stream = Some((header.stream_id, header.sequence.wrapping_add(1)));
        let Some(mut packet) = shared.free.pop() else {
            shared.overflows.fetch_add(1, Ordering::Relaxed);
            continue;
        };
        for (sample, bytes) in packet.samples[..channels * frames]
            .iter_mut()
            .zip(payload.chunks_exact(4))
        {
            let value = f32::from_le_bytes(bytes.try_into().expect("4 bytes"));
            *sample = if value.is_finite() {
                value.clamp(-4.0, 4.0)
            } else {
                0.0
            };
        }
        packet.channels = channels;
        packet.frames = frames;
        shared.received.fetch_add(1, Ordering::Relaxed);
        shared.push(packet);
    }
}

impl Playout {
    /// Read `frames` input frames (at `channels` output channels) into the
    /// scratch buffer; missing audio reads as silence. Returns frames read.
    fn read(&mut self, shared: &ReceiverShared, frames: usize, channels: usize) -> usize {
        let mut read = 0;
        while read < frames {
            if self.current.is_none() {
                self.current = shared.ready.pop();
                self.offset = 0;
            }
            let Some(packet) = self.current.as_ref() else {
                break;
            };
            let available = packet.frames - self.offset;
            let take = available.min(frames - read);
            for frame in 0..take {
                let source = (self.offset + frame) * packet.channels;
                for channel in 0..channels {
                    let from = if packet.channels == 1 {
                        0
                    } else {
                        channel.min(packet.channels - 1)
                    };
                    self.scratch[(read + frame) * channels + channel] =
                        packet.samples[source + from];
                }
            }
            self.offset += take;
            read += take;
            shared.queued_frames.fetch_sub(take, Ordering::AcqRel);
            if self.offset == packet.frames {
                if let Some(done) = self.current.take() {
                    let _ = shared.free.push(done);
                }
            }
        }
        self.scratch[read * channels..frames * channels].fill(0.0);
        read
    }
}

impl AudioCaptureSource for NetworkReceiver {
    /// One quantum whenever one is due at the wall-clock rate. Silence while
    /// the jitter buffer fills or after an underrun, so a Mixer that also
    /// takes live inputs keeps running. When the buffer drifts more than two
    /// quanta from its target, one frame is added or removed per quantum by
    /// linear interpolation (a 0.8 % rate change, inaudible on speech/music).
    fn next_packet_into(
        &self,
        destination: &mut [u8],
        bytes_per_frame: usize,
    ) -> Result<Option<(CapturePacket, usize)>, AudioError> {
        let channels = bytes_per_frame / 4;
        if bytes_per_frame == 0
            || bytes_per_frame % 4 != 0
            || !(1..=MAX_NETWORK_CHANNELS).contains(&channels)
        {
            return Err(AudioError::InvalidFrameSize);
        }
        let quantum =
            audiorouter_engine::PROCESSING_QUANTUM_FRAMES.min(destination.len() / bytes_per_frame);
        if quantum == 0 {
            return Ok(None);
        }
        let Ok(mut playout) = self.playout.try_lock() else {
            return Ok(None);
        };
        let started = *playout.started.get_or_insert_with(Instant::now);
        let rate = u128::from(audiorouter_engine::INTERNAL_SAMPLE_RATE_HZ);
        let due = u64::try_from(started.elapsed().as_nanos() * rate / 1_000_000_000)
            .unwrap_or(u64::MAX)
            .saturating_add(quantum as u64);
        if due.saturating_sub(playout.delivered_frames) > 4_800 {
            playout.delivered_frames = due - 4_800;
        }
        if due < playout.delivered_frames + quantum as u64 {
            return Ok(None);
        }
        playout.delivered_frames += quantum as u64;

        let buffered = self.shared.queued_frames.load(Ordering::Acquire);
        if !playout.playing && buffered >= self.shared.target_frames.load(Ordering::Acquire) {
            playout.playing = true;
        }
        // Far too deep (sender burst after a network stall): skip ahead to
        // the target instead of keeping seconds of delay.
        if playout.playing
            && buffered > self.shared.target_frames.load(Ordering::Acquire) * 4 + 4_800
        {
            while self.shared.queued_frames.load(Ordering::Acquire)
                > self.shared.target_frames.load(Ordering::Acquire)
            {
                let Some(packet) = playout.current.take().or_else(|| self.shared.ready.pop())
                else {
                    break;
                };
                let remaining = packet.frames - playout.offset.min(packet.frames);
                playout.offset = 0;
                self.shared
                    .queued_frames
                    .fetch_sub(remaining, Ordering::AcqRel);
                self.shared.overflows.fetch_add(1, Ordering::Relaxed);
                let _ = self.shared.free.push(packet);
            }
        }
        let bytes = quantum * bytes_per_frame;
        if !playout.playing {
            destination[..bytes].fill(0);
        } else {
            let margin = audiorouter_engine::PROCESSING_QUANTUM_FRAMES * 2;
            let input_frames =
                if buffered > self.shared.target_frames.load(Ordering::Acquire) + margin {
                    quantum + 1
                } else if buffered + margin < self.shared.target_frames.load(Ordering::Acquire) {
                    quantum - 1
                } else {
                    quantum
                };
            let read = playout.read(&self.shared, input_frames, channels);
            if read < input_frames {
                self.shared.underruns.fetch_add(1, Ordering::Relaxed);
                playout.playing = false;
            }
            for frame in 0..quantum {
                // Linear interpolation from `input_frames` onto `quantum`.
                let position =
                    frame as f64 * (input_frames - 1) as f64 / (quantum - 1).max(1) as f64;
                let index = position as usize;
                let next = (index + 1).min(input_frames - 1);
                let fraction = (position - index as f64) as f32;
                for channel in 0..channels {
                    let left = playout.scratch[index * channels + channel];
                    let right = playout.scratch[next * channels + channel];
                    let sample = left + (right - left) * fraction;
                    let offset = (frame * channels + channel) * 4;
                    destination[offset..offset + 4].copy_from_slice(&sample.to_le_bytes());
                }
            }
        }
        Ok(Some((
            CapturePacket {
                frames: quantum as u32,
                flags: 0,
                device_position: 0,
                qpc_position: 0,
            },
            bytes,
        )))
    }
}

fn network_error(error: std::io::Error) -> AudioError {
    AudioError::Network(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use audiorouter_engine::{AudioBlock, AudioTap};

    fn header(sequence: u32) -> NetworkPacketHeader {
        NetworkPacketHeader {
            channels: 2,
            frames: 128,
            sample_rate_hz: 48_000,
            stream_id: 7,
            sequence,
        }
    }

    #[test]
    fn packets_round_trip_and_reject_malformed_or_unsupported_datagrams() {
        let samples: Vec<f32> = (0..256).map(|index| index as f32 / 256.0).collect();
        let mut buffer = [0_u8; MAX_NETWORK_PACKET_BYTES];
        let length = encode_network_packet(header(9), &samples, &mut buffer).unwrap();
        assert_eq!(length, 20 + 256 * 4);
        let (decoded, payload) = decode_network_packet(&buffer[..length]).unwrap();
        assert_eq!(decoded, header(9));
        assert_eq!(
            f32::from_le_bytes(payload[4..8].try_into().unwrap()),
            1.0 / 256.0
        );

        assert_eq!(
            decode_network_packet(&buffer[..length - 1]),
            Err(NetworkAudioError::Malformed)
        );
        assert_eq!(
            decode_network_packet(&buffer[..10]),
            Err(NetworkAudioError::Malformed)
        );
        let mut wrong = buffer;
        wrong[0] = b'X';
        assert_eq!(
            decode_network_packet(&wrong[..length]),
            Err(NetworkAudioError::Malformed)
        );
        let mut rate = buffer;
        rate[8..12].copy_from_slice(&44_100_u32.to_le_bytes());
        assert_eq!(
            decode_network_packet(&rate[..length]),
            Err(NetworkAudioError::Unsupported)
        );
        let mut channels = buffer;
        channels[5] = 8;
        assert_eq!(
            decode_network_packet(&channels[..length]),
            Err(NetworkAudioError::Unsupported)
        );
        assert!(encode_network_packet(header(1), &samples[..10], &mut buffer).is_err());
        assert_eq!(
            network_socket_address("192.168.1.20", 47_800),
            Some("192.168.1.20:47800".parse().unwrap())
        );
        assert_eq!(network_socket_address("streaming-pc", 47_800), None);
        assert_eq!(network_socket_address("192.168.1.20", 0), None);
    }

    fn free_port() -> u16 {
        UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    fn play_quanta(receiver: &NetworkReceiver, quanta: usize) -> Vec<f32> {
        let mut left = Vec::new();
        let mut buffer = vec![0_u8; 128 * 8];
        let deadline = Instant::now() + Duration::from_secs(10);
        while left.len() < quanta * 128 && Instant::now() < deadline {
            match receiver.next_packet_into(&mut buffer, 8).unwrap() {
                Some((packet, bytes)) => {
                    assert_eq!(packet.frames, 128);
                    for frame in buffer[..bytes].chunks_exact(8) {
                        left.push(f32::from_le_bytes(frame[..4].try_into().unwrap()));
                    }
                }
                None => std::thread::sleep(Duration::from_micros(500)),
            }
        }
        left
    }

    /// A tone streamed over loopback UDP plays back continuously once the
    /// jitter buffer fills, and a lost packet is concealed, not skipped.
    #[test]
    fn a_tone_sent_over_loopback_plays_back_continuously() {
        let port = free_port();
        let receiver = NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 20.0).unwrap();
        let sender = NetworkSender::start(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
        let tap = sender.tap();
        let mut block = AudioBlock::new(2, 128).unwrap();
        let tone = |index: usize| {
            (0.25 * (2.0 * std::f64::consts::PI * 997.0 * index as f64 / 48_000.0).sin()) as f32
        };
        // Feed 1 s of tone at real time from a separate thread.
        let feeder = std::thread::spawn(move || {
            let start = Instant::now();
            for quantum in 0..375 {
                for channel in 0..2 {
                    for (frame, sample) in
                        block.channel_mut(channel).unwrap().iter_mut().enumerate()
                    {
                        *sample = tone(quantum * 128 + frame);
                    }
                }
                tap.on_processed_block(0, &block);
                let due = start + Duration::from_micros((quantum as u64 + 1) * 2_667);
                if let Some(wait) = due.checked_duration_since(Instant::now()) {
                    std::thread::sleep(wait);
                }
            }
            sender
        });
        let played = play_quanta(&receiver, 300);
        let sender = feeder.join().unwrap();
        assert_eq!(sender.stats().dropped_packets, 0);
        let first = played
            .iter()
            .position(|sample| sample.abs() > 0.05)
            .expect("tone arrives");
        let steady = &played[first + 64..];
        let coefficient = (2.0 * (2.0 * std::f64::consts::PI * 997.0 / 48_000.0).cos()) as f32;
        let jumps = steady
            .windows(3)
            .filter(|window| (window[2] - (coefficient * window[1] - window[0])).abs() > 0.02)
            .count();
        let stats = receiver.stats();
        eprintln!(
            "loopback tone: {jumps} discontinuities in {} samples; {stats:?}",
            steady.len()
        );
        assert_eq!(stats.rejected_datagrams, 0);
        assert!(stats.received_packets > 250, "{stats:?}");
        // Loopback UDP on an idle machine loses nothing; allow scheduling
        // hiccups of this unprivileged test thread, not systematic damage.
        assert!(jumps <= 2, "{jumps} discontinuities, {stats:?}");
    }

    #[test]
    fn datagrams_from_another_address_are_rejected() {
        let port = free_port();
        // Expect a sender that is not this machine's loopback address.
        let receiver = NetworkReceiver::start("127.0.0.2".parse().unwrap(), port, 20.0).unwrap();
        let sender = NetworkSender::start(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
        let block = AudioBlock::new(2, 128).unwrap();
        for _ in 0..20 {
            sender.tap().on_processed_block(0, &block);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stats().rejected_datagrams < 20 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let stats = receiver.stats();
        assert_eq!(stats.rejected_datagrams, 20);
        assert_eq!(stats.received_packets, 0);
        // The UI can name the address audio really comes from.
        assert_eq!(
            stats.last_rejected_sender,
            Some("127.0.0.1".parse().unwrap())
        );
        // Arbitrary traffic from a stranger never becomes that hint.
        let other =
            NetworkReceiver::start("127.0.0.2".parse().unwrap(), free_port(), 20.0).unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .send_to(
                b"not audio",
                other
                    .listen_address()
                    .to_string()
                    .replace("0.0.0.0", "127.0.0.1"),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while other.stats().rejected_datagrams < 1 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(other.stats().rejected_datagrams, 1);
        assert_eq!(other.stats().last_rejected_sender, None);
    }

    /// Play `quanta` stereo quanta at the receiver's wall-clock pace and
    /// return interleaved (left, right) frames.
    fn play_stereo(receiver: &NetworkReceiver, quanta: usize) -> Vec<(f32, f32)> {
        let mut frames = Vec::new();
        let mut buffer = vec![0_u8; 128 * 8];
        let deadline = Instant::now() + Duration::from_secs(30);
        while frames.len() < quanta * 128 && Instant::now() < deadline {
            match receiver.next_packet_into(&mut buffer, 8).unwrap() {
                Some((_, bytes)) => {
                    for frame in buffer[..bytes].chunks_exact(8) {
                        frames.push((
                            f32::from_le_bytes(frame[..4].try_into().unwrap()),
                            f32::from_le_bytes(frame[4..].try_into().unwrap()),
                        ));
                    }
                }
                None => std::thread::sleep(Duration::from_micros(300)),
            }
        }
        frames
    }

    /// Feed `quanta` blocks of a 997 Hz tone (left) and its negation (right)
    /// through `tap` at `speed` times real time from a separate thread.
    fn feed_tone(
        tap: NetworkSendTap,
        quanta: usize,
        speed: f64,
        channels: usize,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut block = AudioBlock::new(channels, 128).unwrap();
            let start = Instant::now();
            let quantum = Duration::from_secs_f64(128.0 / 48_000.0 / speed);
            for index in 0..quanta {
                for channel in 0..channels {
                    let sign = if channel == 0 { 1.0 } else { -1.0 };
                    for (frame, sample) in
                        block.channel_mut(channel).unwrap().iter_mut().enumerate()
                    {
                        let n = (index * 128 + frame) as f64;
                        *sample = (sign
                            * 0.25
                            * (2.0 * std::f64::consts::PI * 997.0 * n / 48_000.0).sin())
                            as f32;
                    }
                }
                tap.on_processed_block(0, &block);
                if let Some(wait) =
                    (start + quantum * (index as u32 + 1)).checked_duration_since(Instant::now())
                {
                    std::thread::sleep(wait);
                }
            }
        })
    }

    #[test]
    fn stereo_stays_separate_and_a_mono_sender_plays_on_both_channels() {
        for channels in [2_usize, 1] {
            let port = free_port();
            let receiver =
                NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 20.0).unwrap();
            let sender = NetworkSender::start(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
            let feeder = feed_tone(sender.tap(), 200, 1.0, channels);
            let frames = play_stereo(&receiver, 150);
            feeder.join().unwrap();
            let loud = frames
                .iter()
                .filter(|(left, _)| left.abs() > 0.1)
                .collect::<Vec<_>>();
            assert!(
                loud.len() > 5_000,
                "{channels} ch: tone arrives ({})",
                loud.len()
            );
            for (left, right) in loud {
                if channels == 2 {
                    assert!(
                        (left + right).abs() < 1e-6,
                        "right is the negated left: {left} {right}"
                    );
                } else {
                    assert_eq!(left, right, "mono plays identically on both channels");
                }
            }
        }
    }

    /// Two computers never share a clock. A sender running 0.3 % fast or
    /// slow must neither starve nor flood the receiver: the buffer stays
    /// near its target, with no gap and no skip, for the whole run.
    #[test]
    fn sender_clock_drift_is_absorbed_without_gaps_or_growing_delay() {
        let runs = [1.003_f64, 0.997].map(|speed| {
            std::thread::spawn(move || {
                let port = free_port();
                let receiver =
                    NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 40.0).unwrap();
                let sender =
                    NetworkSender::start(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
                // 6 s of audio: 18 ms of drift each way without correction.
                let feeder = feed_tone(sender.tap(), 2_250, speed, 2);
                // Sample the delay (buffer depth) between 1 s and 5 s.
                let (frames, depth) = std::thread::scope(|scope| {
                    let monitor = scope.spawn(|| {
                        std::thread::sleep(Duration::from_secs(1));
                        let mut depth = (f64::MAX, 0.0_f64);
                        for _ in 0..40 {
                            let ms = receiver.stats().buffered_frames as f64 * 1_000.0 / 48_000.0;
                            depth = (depth.0.min(ms), depth.1.max(ms));
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        depth
                    });
                    let frames = play_stereo(&receiver, 2_150);
                    (frames, monitor.join().unwrap())
                });
                feeder.join().unwrap();
                assert!(
                    depth.0 >= 20.0 && depth.1 <= 60.0,
                    "{speed}: delay stayed near 40 ms: {depth:?}"
                );
                let stats = receiver.stats();
                let first = frames
                    .iter()
                    .position(|(left, _)| left.abs() > 0.05)
                    .expect("tone arrives");
                let steady = &frames[first + 256..frames.len().saturating_sub(4_800)];
                let coefficient =
                    (2.0 * (2.0 * std::f64::consts::PI * 997.0 / 48_000.0).cos()) as f32;
                // Drift correction moves one frame per quantum: a 0.8 %
                // pitch nudge, never a step. Gaps or skips would be steps.
                let steps = steady
                    .windows(3)
                    .filter(|window| {
                        (window[2].0 - (coefficient * window[1].0 - window[0].0)).abs() > 0.05
                    })
                    .count();
                (speed, steps, stats)
            })
        });
        for run in runs {
            let (speed, steps, stats) = run.join().unwrap();
            eprintln!("drift {speed}: {steps} steps; {stats:?}");
            assert_eq!(stats.lost_packets, 0, "{speed}: {stats:?}");
            assert_eq!(
                stats.overflow_packets, 0,
                "{speed}: no skip-ahead: {stats:?}"
            );
            assert!(
                stats.underruns <= 1,
                "{speed}: at most the final drain: {stats:?}"
            );
            assert!(steps <= 2, "{speed}: {steps} audible steps");
        }
    }

    /// A sender that restarts (new stream id, sequence back at 0) is followed
    /// at once instead of being treated as late or duplicate audio.
    #[test]
    fn a_restarted_sender_is_followed_immediately() {
        let port = free_port();
        let receiver = NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 10.0).unwrap();
        let block = AudioBlock::new(2, 128).unwrap();
        for _ in 0..2 {
            let sender = NetworkSender::start(SocketAddr::from(([127, 0, 0, 1], port))).unwrap();
            for _ in 0..30 {
                sender.tap().on_processed_block(0, &block);
                std::thread::sleep(Duration::from_millis(1));
            }
            let deadline = Instant::now() + Duration::from_secs(5);
            while sender.stats().sent_packets < 30 && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stats().received_packets < 60 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let stats = receiver.stats();
        assert_eq!(stats.received_packets, 60, "{stats:?}");
        assert_eq!(stats.late_packets, 0, "{stats:?}");
    }

    /// After a Wi-Fi stall the sender's queued audio arrives as a burst. The
    /// receiver drops the excess instead of keeping seconds of extra delay.
    #[test]
    fn a_burst_after_a_stall_does_not_leave_extra_delay() {
        let port = free_port();
        let receiver = NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 40.0).unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let samples = vec![0.1_f32; 256];
        let mut buffer = [0_u8; MAX_NETWORK_PACKET_BYTES];
        let mut send = |sequence: u32| {
            let length = encode_network_packet(header(sequence), &samples, &mut buffer).unwrap();
            socket
                .send_to(&buffer[..length], ("127.0.0.1", port))
                .unwrap();
        };
        for sequence in 0..30 {
            send(sequence);
        }
        let _ = play_stereo(&receiver, 20);
        // One second of audio at once.
        for sequence in 30..405 {
            send(sequence);
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stats().received_packets < 405 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        let _ = play_stereo(&receiver, 4);
        let stats = receiver.stats();
        let buffered_ms = stats.buffered_frames as f64 * 1_000.0 / 48_000.0;
        assert!(
            stats.overflow_packets > 0,
            "the excess was dropped: {stats:?}"
        );
        assert!(
            buffered_ms <= 40.0 + 10.0,
            "delay back near the 40 ms target, not {buffered_ms} ms"
        );
    }

    /// The real two-computer path: audio addressed to this computer's LAN
    /// address (not loopback), and IPv6. Skips the LAN half without a network.
    #[test]
    fn audio_arrives_over_this_computers_lan_address_and_over_ipv6() {
        let lan = UdpSocket::bind("0.0.0.0:0")
            .and_then(|probe| probe.connect("192.0.2.1:9").map(|()| probe))
            .and_then(|probe| probe.local_addr())
            .map(|address| address.ip())
            .ok()
            .filter(|ip| !ip.is_loopback() && !ip.is_unspecified());
        let mut addresses: Vec<IpAddr> = vec!["::1".parse().unwrap()];
        match lan {
            Some(ip) => addresses.push(ip),
            None => eprintln!("no LAN address; only IPv6 loopback checked"),
        }
        for address in addresses {
            let port = free_port();
            let receiver = NetworkReceiver::start(address, port, 20.0).unwrap();
            let sender = NetworkSender::start(SocketAddr::new(address, port)).unwrap();
            let feeder = feed_tone(sender.tap(), 120, 1.0, 2);
            let frames = play_stereo(&receiver, 80);
            feeder.join().unwrap();
            let stats = receiver.stats();
            eprintln!("{address}: {stats:?}");
            assert_eq!(stats.rejected_datagrams, 0, "{address}: {stats:?}");
            assert!(stats.received_packets >= 100, "{address}: {stats:?}");
            assert!(
                frames.iter().any(|(left, _)| left.abs() > 0.2),
                "{address}: tone plays"
            );
        }
    }

    #[test]
    fn a_sequence_gap_is_concealed_and_late_packets_are_dropped() {
        let port = free_port();
        let receiver = NetworkReceiver::start("127.0.0.1".parse().unwrap(), port, 10.0).unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let samples = vec![0.5_f32; 256];
        let mut buffer = [0_u8; MAX_NETWORK_PACKET_BYTES];
        for sequence in [0_u32, 1, 4, 3, 5] {
            let length = encode_network_packet(header(sequence), &samples, &mut buffer).unwrap();
            socket
                .send_to(&buffer[..length], ("127.0.0.1", port))
                .unwrap();
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        while receiver.stats().received_packets + receiver.stats().late_packets < 5
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        let stats = receiver.stats();
        assert_eq!(stats.received_packets, 4, "{stats:?}");
        assert_eq!(stats.lost_packets, 2, "sequences 2 and 3 were concealed");
        assert_eq!(stats.late_packets, 1, "sequence 3 arrived after 4");
        // 4 received + 2 concealed quanta are buffered for playout.
        assert_eq!(stats.buffered_frames, 6 * 128);
    }
}
