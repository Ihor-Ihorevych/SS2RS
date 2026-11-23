//! Server metrics in the Prometheus text format, and a tiny HTTP endpoint that serves them
//! (roadmap T08-020).
//!
//! The endpoint is deliberately small: it answers `GET /metrics` and `GET /healthz` on one
//! thread, one connection at a time, with a short read timeout and a size limit on the request, and
//! nothing else. It is **off unless `metrics_bind` is set**, and an operator who exposes it beyond
//! `127.0.0.1` should put it behind something that authenticates.

use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

/// The numbers the server publishes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Metrics {
    /// Seconds since the loop started.
    pub uptime_seconds: f64,
    /// Simulation ticks run.
    pub tick: u64,
    /// The tick rate.
    pub tick_hz: u32,
    /// Players connected.
    pub players: u32,
    /// The player limit.
    pub max_players: u32,
    /// How long the last tick took, in microseconds.
    pub tick_micros_last: u64,
    /// The longest tick so far, in microseconds.
    pub tick_micros_max: u64,
    /// Ticks that were due but given up because the server fell behind.
    pub ticks_dropped: u64,
}

/// The metrics in the Prometheus text format.
pub fn render(m: &Metrics) -> String {
    let mut out = String::new();
    let mut metric = |name: &str, kind: &str, help: &str, value: String| {
        let _ = writeln!(
            out,
            "# HELP {name} {help}\n# TYPE {name} {kind}\n{name} {value}"
        );
    };
    metric(
        "ss2_server_uptime_seconds",
        "gauge",
        "Seconds since the server loop started.",
        m.uptime_seconds.to_string(),
    );
    metric(
        "ss2_server_tick",
        "counter",
        "Simulation ticks run.",
        m.tick.to_string(),
    );
    metric(
        "ss2_server_tick_rate_hz",
        "gauge",
        "Configured simulation ticks per second.",
        m.tick_hz.to_string(),
    );
    metric(
        "ss2_server_players",
        "gauge",
        "Players connected.",
        m.players.to_string(),
    );
    metric(
        "ss2_server_max_players",
        "gauge",
        "Player limit.",
        m.max_players.to_string(),
    );
    metric(
        "ss2_server_tick_duration_microseconds",
        "gauge",
        "Duration of the last tick.",
        m.tick_micros_last.to_string(),
    );
    metric(
        "ss2_server_tick_duration_max_microseconds",
        "gauge",
        "Longest tick so far.",
        m.tick_micros_max.to_string(),
    );
    metric(
        "ss2_server_ticks_dropped_total",
        "counter",
        "Ticks given up because the server fell behind.",
        m.ticks_dropped.to_string(),
    );
    out
}

const MAX_REQUEST_BYTES: usize = 8 * 1024;
const READ_TIMEOUT: Duration = Duration::from_millis(500);

/// The running endpoint; stop it with [`MetricsServer::stop`].
pub struct MetricsServer {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body.as_bytes());
    let _ = stream.flush();
}

fn serve(mut stream: TcpStream, shared: &Mutex<Metrics>) {
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let _ = stream.set_write_timeout(Some(READ_TIMEOUT));
    let mut request = Vec::new();
    let mut chunk = [0_u8; 1024];
    // Read until the end of the headers, the size limit or the timeout.
    while !request.windows(4).any(|w| w == b"\r\n\r\n") {
        if request.len() > MAX_REQUEST_BYTES {
            return respond(
                &mut stream,
                "431 Request Header Fields Too Large",
                "text/plain",
                "request too large\n",
            );
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => request.extend_from_slice(&chunk[..n]),
        }
    }
    let line_end = request
        .windows(2)
        .position(|w| w == b"\r\n")
        .unwrap_or(request.len());
    let line = String::from_utf8_lossy(&request[..line_end]).into_owned();
    let mut parts = line.split_whitespace();
    let (Some(method), Some(path), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return respond(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            "bad request\n",
        );
    };
    if !version.starts_with("HTTP/") || !method.chars().all(|c| c.is_ascii_uppercase()) {
        return respond(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            "bad request\n",
        );
    }
    if method != "GET" {
        return respond(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain",
            "only GET\n",
        );
    }
    match path {
        "/metrics" => {
            let text = render(&shared.lock().map(|m| m.clone()).unwrap_or_default());
            respond(&mut stream, "200 OK", "text/plain; version=0.0.4", &text);
        }
        "/healthz" => respond(&mut stream, "200 OK", "text/plain", "ok\n"),
        _ => respond(&mut stream, "404 Not Found", "text/plain", "not found\n"),
    }
}

impl MetricsServer {
    /// Listen on `addr` (an IP address and port; port 0 picks a free one) and serve `shared`.
    pub fn start(addr: &str, shared: Arc<Mutex<Metrics>>) -> io::Result<Self> {
        let addr: SocketAddr = addr.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{addr:?} is not an IP address and port"),
            )
        })?;
        let listener = TcpListener::bind(addr)?;
        let addr = listener.local_addr()?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("ss2-metrics".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if flag.load(Ordering::SeqCst) {
                        break;
                    }
                    if let Ok(stream) = stream {
                        serve(stream, &shared);
                    }
                }
            })?;
        Ok(Self {
            addr,
            stop,
            thread: Some(thread),
        })
    }

    /// The address it listens on (with the real port when 0 was asked for).
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }

    /// Stop serving and release the port.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the accept loop so that it sees the flag.
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(500));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for MetricsServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn sample() -> Metrics {
        Metrics {
            uptime_seconds: 12.5,
            tick: 375,
            tick_hz: 30,
            players: 3,
            max_players: 8,
            tick_micros_last: 410,
            tick_micros_max: 2_300,
            ticks_dropped: 2,
        }
    }

    fn get(addr: std::net::SocketAddr, request: &str) -> String {
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn the_text_format_has_help_type_and_a_value_for_every_metric() {
        let text = render(&sample());
        for (name, value) in [
            ("ss2_server_uptime_seconds", "12.5"),
            ("ss2_server_tick", "375"),
            ("ss2_server_tick_rate_hz", "30"),
            ("ss2_server_players", "3"),
            ("ss2_server_max_players", "8"),
            ("ss2_server_tick_duration_microseconds", "410"),
            ("ss2_server_tick_duration_max_microseconds", "2300"),
            ("ss2_server_ticks_dropped_total", "2"),
        ] {
            assert!(text.contains(&format!("# HELP {name} ")), "{name}");
            assert!(text.contains(&format!("# TYPE {name} ")), "{name}");
            assert!(
                text.lines().any(|l| l == format!("{name} {value}")),
                "{name} {value} in\n{text}"
            );
        }
        assert!(text.ends_with('\n'));
        assert!(text.contains("# TYPE ss2_server_ticks_dropped_total counter"));
        assert!(text.contains("# TYPE ss2_server_players gauge"));
    }

    #[test]
    fn the_endpoint_serves_metrics_health_and_not_found() {
        let shared = Arc::new(Mutex::new(sample()));
        let server = MetricsServer::start("127.0.0.1:0", Arc::clone(&shared)).unwrap();
        let addr = server.local_addr();
        let ok = get(addr, "GET /metrics HTTP/1.1\r\nHost: x\r\n\r\n");
        assert!(ok.starts_with("HTTP/1.1 200 OK"), "{ok}");
        assert!(
            ok.contains("Content-Type: text/plain; version=0.0.4"),
            "{ok}"
        );
        assert!(ok.contains("ss2_server_players 3"), "{ok}");
        // It shows the current values, not the first ones.
        shared.lock().unwrap().players = 5;
        assert!(get(addr, "GET /metrics HTTP/1.1\r\n\r\n").contains("ss2_server_players 5"));
        let health = get(addr, "GET /healthz HTTP/1.1\r\n\r\n");
        assert!(
            health.starts_with("HTTP/1.1 200 OK") && health.ends_with("ok\n"),
            "{health}"
        );
        let missing = get(addr, "GET /admin HTTP/1.1\r\n\r\n");
        assert!(missing.starts_with("HTTP/1.1 404"), "{missing}");
        let wrong_method = get(addr, "POST /metrics HTTP/1.1\r\nContent-Length: 0\r\n\r\n");
        assert!(wrong_method.starts_with("HTTP/1.1 405"), "{wrong_method}");
        server.stop();
    }

    #[test]
    fn garbage_and_slow_clients_do_not_stop_the_endpoint() {
        let server = MetricsServer::start("127.0.0.1:0", Arc::new(Mutex::new(sample()))).unwrap();
        let addr = server.local_addr();
        // Bytes that are not HTTP, a request with no end, and a huge request line.
        assert!(get(addr, "\u{0}\u{1}\u{2} garbage\r\n\r\n").starts_with("HTTP/1.1 400"));
        let huge = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(100_000));
        // The server answers 431 and closes without reading the rest, so the client may see a reset instead.
        if let Ok(mut stream) = TcpStream::connect(addr) {
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let _ = stream.write_all(huge.as_bytes());
            let mut response = String::new();
            if stream.read_to_string(&mut response).is_ok() && !response.is_empty() {
                assert!(
                    response.starts_with("HTTP/1.1 431"),
                    "refused as too large, not served: {response}"
                );
            }
        }
        let mut silent = TcpStream::connect(addr).unwrap();
        silent
            .write_all(b"GET /metrics HTTP/1.1\r\nHost: x\r\n")
            .unwrap(); // never finishes the headers
        std::thread::sleep(Duration::from_millis(50));
        // Another client is served after the slow one times out (the endpoint reads with a short timeout).
        let ok = get(addr, "GET /healthz HTTP/1.1\r\n\r\n");
        assert!(ok.starts_with("HTTP/1.1 200"), "{ok}");
        drop(silent);
        server.stop();
    }

    #[test]
    fn stopping_releases_the_port() {
        let server = MetricsServer::start("127.0.0.1:0", Arc::new(Mutex::new(sample()))).unwrap();
        let addr = server.local_addr();
        server.stop();
        assert!(
            TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_err(),
            "nothing listens any more"
        );
    }

    #[test]
    fn a_bad_address_is_an_error_not_a_panic() {
        assert!(MetricsServer::start("not an address", Arc::new(Mutex::new(sample()))).is_err());
    }
}
