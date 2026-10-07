//! Connection intake for the HTTP adapter: blocking accept per listener, a
//! bounded worker queue, a 503 for overflow, and a prompt stop. Uses only
//! `std` and `serde_json`, so its tests also run off Windows; request parsing,
//! tokens and routing stay in `http_api`.
use serde_json::json;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    mpsc::{SyncSender, TrySendError},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

/// Four workers plus 32 waiting sockets bound memory and concurrency while
/// accommodating the frontend's initial discovery/read burst.
pub const WORKERS: usize = 4;
pub const QUEUE: usize = 32;
/// A connection refused because the queue is full gets its 503 within these
/// bounds, so a slow or silent client cannot hold the accept thread.
const BUSY_WRITE_TIMEOUT: Duration = Duration::from_millis(100);
const BUSY_LINGER: Duration = Duration::from_millis(20);
const BUSY_DISCARD: usize = 64 * 1024;
/// Upper bound for the self-connection that wakes a blocked accept on stop.
const WAKE_TIMEOUT: Duration = Duration::from_millis(250);

/// Serves one handler call per accepted connection.
pub type Handler = dyn Fn(&mut TcpStream, &AtomicBool) + Send + Sync;
/// Decides whether a peer may use a listener; the flag marks the network
/// (non-loopback) listener.
pub type PeerFilter = fn(IpAddr, bool) -> bool;

pub struct ConnectionPool {
    stop: Arc<AtomicBool>,
    /// One blocking accept thread per listener, with the address that wakes it.
    acceptors: Vec<(SocketAddr, std::thread::JoinHandle<()>)>,
    workers: Vec<std::thread::JoinHandle<()>>,
    /// Accept calls made by all listener threads. An idle pool makes one per
    /// listener and then waits in the kernel.
    #[cfg_attr(not(test), allow(dead_code))]
    accept_calls: Arc<AtomicUsize>,
}

impl Drop for ConnectionPool {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let mut all_stopped = true;
        for (address, thread) in self.acceptors.drain(..) {
            // std cannot cancel a blocked accept, and closing a socket that
            // another thread is waiting on is not sound. A connection to the
            // listener completes that accept (on Windows as elsewhere); the
            // thread then sees `stop` and drops both sockets.
            let woken = TcpStream::connect_timeout(&address, WAKE_TIMEOUT).is_ok();
            if woken || thread.is_finished() {
                let _ = thread.join();
            } else {
                // The network address may have left this PC. That thread
                // exits on its next accept or error; never hang the caller.
                all_stopped = false;
            }
        }
        // Workers wait in `recv` until every accept thread has dropped its
        // queue sender, so join them only when that is certain.
        if all_stopped {
            for worker in self.workers.drain(..) {
                let _ = worker.join();
            }
        }
    }
}

impl ConnectionPool {
    /// Listeners must be in blocking mode (the `TcpListener::bind` default).
    pub fn start(
        listeners: Vec<(TcpListener, bool)>,
        peer_allowed: PeerFilter,
        handler: Arc<Handler>,
    ) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let accept_calls = Arc::new(AtomicUsize::new(0));
        // Dropping `pool` after a failed spawn stops whatever already started.
        // It is declared before the queue so that, on an early return, the
        // local sender is dropped first and the workers' `recv` can end.
        let mut pool = Self {
            stop: stop.clone(),
            acceptors: Vec::new(),
            workers: Vec::new(),
            accept_calls: accept_calls.clone(),
        };
        let (sender, receiver) = std::sync::mpsc::sync_channel::<TcpStream>(QUEUE);
        let receiver = Arc::new(Mutex::new(receiver));
        for _ in 0..WORKERS {
            let (receiver, handler, stop) = (receiver.clone(), handler.clone(), stop.clone());
            let worker = std::thread::Builder::new()
                .name("audiorouter-http-worker".into())
                .spawn(move || loop {
                    // Waits until a socket is queued, or until every accept
                    // thread has exited and dropped its sender.
                    let stream = {
                        let Ok(receiver) = receiver.lock() else { break };
                        receiver.recv()
                    };
                    let Ok(mut stream) = stream else { break };
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    handler(&mut stream, &stop);
                })
                .map_err(|_| "Cannot start HTTP adapter")?;
            pool.workers.push(worker);
        }
        for (listener, network) in listeners {
            let address = listener.local_addr().map_err(|_| "Cannot read API port")?;
            let (sender, stop, accept_calls) = (sender.clone(), stop.clone(), accept_calls.clone());
            let acceptor = std::thread::Builder::new()
                .name("audiorouter-http".into())
                .spawn(move || {
                    accept_loop(
                        &listener,
                        network,
                        peer_allowed,
                        &sender,
                        &stop,
                        &accept_calls,
                    )
                })
                .map_err(|_| "Cannot start HTTP adapter")?;
            pool.acceptors.push((address, acceptor));
        }
        // From here only the accept threads hold senders.
        drop(sender);
        Ok(pool)
    }

    #[cfg(test)]
    pub fn accept_calls(&self) -> usize {
        self.accept_calls.load(Ordering::Relaxed)
    }
}

/// Blocking accept for one listener. Returns when `stop` is set (the stopper
/// connects to wake it) or when the listener fails permanently.
fn accept_loop(
    listener: &TcpListener,
    network: bool,
    peer_allowed: PeerFilter,
    queue: &SyncSender<TcpStream>,
    stop: &AtomicBool,
    accept_calls: &AtomicUsize,
) {
    loop {
        accept_calls.fetch_add(1, Ordering::Relaxed);
        let accepted = listener.accept();
        if stop.load(Ordering::Acquire) {
            return;
        }
        match accepted {
            Ok((stream, address)) => {
                if !peer_allowed(address.ip(), network) {
                    continue;
                }
                match queue.try_send(stream) {
                    Ok(()) => {}
                    Err(TrySendError::Full(mut stream)) => {
                        let _ = reject_busy(&mut stream);
                    }
                    Err(TrySendError::Disconnected(_)) => return,
                }
            }
            // A client that resets before accept completes is not a listener
            // failure.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionAborted
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return,
        }
    }
}

/// Answers a connection the full queue cannot take with 503 and
/// `Retry-After`, bounded by `BUSY_WRITE_TIMEOUT` plus `BUSY_LINGER` so the
/// client cannot hold the accept thread.
fn reject_busy(stream: &mut TcpStream) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_write_timeout(Some(BUSY_WRITE_TIMEOUT))?;
    reply_with(
        stream,
        503,
        "application/json",
        "Retry-After: 1\r\n",
        json!({"error": {"message": "AudioRouter API is busy; retry in a second"}})
            .to_string()
            .as_bytes(),
    )?;
    close_gracefully(stream, BUSY_LINGER, BUSY_LINGER, BUSY_DISCARD);
    Ok(())
}

/// Writes a complete `Connection: close` response. `extra` holds zero or
/// more complete `Name: value\r\n` header lines.
pub fn reply_with(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    extra: &str,
    body: &[u8],
) -> std::io::Result<()> {
    write!(stream, "HTTP/1.1 {status} {}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n{extra}Connection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n\r\n", match status { 200 => "OK", 400 => "Bad Request", 401 => "Unauthorized", 403 => "Forbidden", 404 => "Not Found", 405 => "Method Not Allowed", 409 => "Conflict", 413 => "Payload Too Large", 429 => "Too Many Requests", _ => "Service Unavailable" }, body.len())?;
    stream.write_all(body)
}

/// Finishes the response before a bounded discard of already arriving bytes.
/// Closing a Windows socket with unread request data can reset the
/// connection and hide the actionable error response from HTTP clients.
pub fn close_gracefully(
    stream: &mut TcpStream,
    read_timeout: Duration,
    total: Duration,
    limit: usize,
) {
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = stream.set_read_timeout(Some(read_timeout));
    let started = Instant::now();
    let mut discarded = 0usize;
    let mut buffer = [0u8; 4096];
    while discarded < limit && started.elapsed() < total {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => discarded += count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::net::Ipv4Addr;
    use std::sync::{mpsc, Condvar};

    fn loopback_only(peer: IpAddr, _network: bool) -> bool {
        peer.is_loopback()
    }
    /// A pool on an ephemeral loopback port whose handler reports each call
    /// on `entered`, waits until `release` is set, then answers 200.
    fn pool(release: Arc<(Mutex<bool>, Condvar)>) -> (ConnectionPool, u16, mpsc::Receiver<()>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let (entered, calls) = mpsc::channel();
        let entered = Mutex::new(entered);
        let handler: Arc<Handler> = Arc::new(move |stream, _stop| {
            let _ = entered.lock().unwrap().send(());
            let (open, wake) = &*release;
            let mut open = open.lock().unwrap();
            while !*open {
                open = wake.wait(open).unwrap();
            }
            let _ = reply_with(stream, 200, "application/json", "", b"{}");
        });
        let pool = ConnectionPool::start(vec![(listener, false)], loopback_only, handler).unwrap();
        (pool, port, calls)
    }
    fn connect(port: u16) -> TcpStream {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream
    }
    fn wait_for(condition: impl Fn() -> bool) -> bool {
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(5) {
            if condition() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        false
    }

    #[test]
    fn full_queue_answers_503_with_retry_after_instead_of_dropping() {
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let (pool, port, calls) = pool(release.clone());
        let busy = (0..WORKERS).map(|_| connect(port)).collect::<Vec<_>>();
        for _ in 0..WORKERS {
            calls.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        // Fill the queue; no worker can take these sockets yet.
        let queued = (0..QUEUE).map(|_| connect(port)).collect::<Vec<_>>();
        assert!(wait_for(|| pool.accept_calls() > WORKERS + QUEUE));
        // The next connection is answered at once instead of hanging.
        let started = Instant::now();
        let mut overflow = connect(port);
        let mut response = String::new();
        overflow.read_to_string(&mut response).unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        let (headers, body) = response.split_once("\r\n\r\n").unwrap();
        assert!(
            headers.starts_with("HTTP/1.1 503 Service Unavailable\r\n"),
            "{headers}"
        );
        assert!(headers.contains("\r\nRetry-After: 1\r\n"), "{headers}");
        assert!(headers.contains("\r\nConnection: close\r\n"), "{headers}");
        let body: Value = serde_json::from_str(body).unwrap();
        assert!(body["error"]["message"].as_str().unwrap().contains("busy"));
        // A client that sends its request first also gets the 503.
        assert!(wait_for(|| pool.accept_calls() > WORKERS + QUEUE + 1));
        let mut eager = connect(port);
        eager
            .write_all(b"GET /api/v1/status HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        eager.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 503"), "{response}");
        // Releasing the handler completes every held and queued connection.
        *release.0.lock().unwrap() = true;
        release.1.notify_all();
        for mut stream in busy.into_iter().chain(queued) {
            let mut response = String::new();
            stream.read_to_string(&mut response).unwrap();
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        }
        let started = Instant::now();
        drop(pool);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn stop_wakes_a_blocked_accept_promptly_and_frees_the_port() {
        let release = Arc::new((Mutex::new(true), Condvar::new()));
        let (pool, port, _calls) = pool(release);
        // Let the accept thread and every worker block.
        assert!(wait_for(|| pool.accept_calls() == 1));
        std::thread::sleep(Duration::from_millis(50));
        let started = Instant::now();
        drop(pool);
        let elapsed = started.elapsed();
        assert!(elapsed < Duration::from_millis(500), "{elapsed:?}");
        // Every thread has exited and the listener is closed.
        drop(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).unwrap());
    }

    #[test]
    fn idle_accept_waits_in_the_kernel_instead_of_polling() {
        let release = Arc::new((Mutex::new(true), Condvar::new()));
        let (pool, port, _calls) = pool(release);
        // A 10 ms polling loop would make about 30 accept calls here.
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(pool.accept_calls(), 1);
        let mut stream = connect(port);
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(pool.accept_calls(), 2);
    }

    #[test]
    fn refused_peers_are_closed_without_reaching_a_worker() {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handled = Arc::new(AtomicUsize::new(0));
        let counter = handled.clone();
        let pool = ConnectionPool::start(
            vec![(listener, false)],
            |_, _| false,
            Arc::new(move |_, _| {
                counter.fetch_add(1, Ordering::Relaxed);
            }),
        )
        .unwrap();
        let mut stream = connect(port);
        let mut response = Vec::new();
        let _ = stream.read_to_end(&mut response);
        assert!(response.is_empty());
        drop(pool);
        assert_eq!(handled.load(Ordering::Relaxed), 0);
    }
}
