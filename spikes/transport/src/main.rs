//! Throwaway loopback benchmark behind RFC 0006 (the network transport, T08-005).
//!
//! Loopback measures the cost of the protocol stack and the runtime, **not** the network: there is
//! no loss, no jitter and no real latency. It answers "what does QUIC cost on top of plain UDP,
//! for small game-sized messages", not "which behaves better on a bad link".
//!
//! Measured (64-byte ping-pong, many round trips, percentiles in microseconds):
//!  * plain UDP, blocking sockets on two threads (the floor),
//!  * plain UDP through tokio (separates the runtime from the protocol),
//!  * QUIC unreliable datagrams (`quinn`, RFC 9221),
//!  * QUIC reliable stream (`quinn`),
//! plus the QUIC handshake time, the maximum datagram size it allows, and the bytes it put on the
//! wire per application datagram (from the connection's own counters).
//!
//! Run: `cargo run --release` in this folder.

use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};

const ROUNDS: usize = 20_000;
const WARMUP: usize = 500;
const PAYLOAD: usize = 64;

fn report(name: &str, mut samples: Vec<f64>) {
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |p: f64| samples[((samples.len() - 1) as f64 * p) as usize];
    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    println!(
        "{name:<34} mean {mean:>7.1}   p50 {:>7.1}   p99 {:>7.1}   p99.9 {:>8.1}   max {:>8.1}",
        pct(0.5),
        pct(0.99),
        pct(0.999),
        samples[samples.len() - 1]
    );
}

fn plain_udp_blocking() -> Vec<f64> {
    let server = UdpSocket::bind("127.0.0.1:0").unwrap();
    let server_addr = server.local_addr().unwrap();
    let handle = std::thread::spawn(move || {
        let mut buf = [0_u8; 2048];
        for _ in 0..ROUNDS + WARMUP {
            let (n, from) = server.recv_from(&mut buf).unwrap();
            server.send_to(&buf[..n], from).unwrap();
        }
    });
    let client = UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut buf = [0_u8; 2048];
    let message = [7_u8; PAYLOAD];
    let mut samples = Vec::with_capacity(ROUNDS);
    for i in 0..ROUNDS + WARMUP {
        let t = Instant::now();
        client.send_to(&message, server_addr).unwrap();
        client.recv_from(&mut buf).unwrap();
        if i >= WARMUP {
            samples.push(t.elapsed().as_secs_f64() * 1e6);
        }
    }
    handle.join().unwrap();
    samples
}

async fn plain_udp_tokio() -> Vec<f64> {
    let server = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let server_addr = server.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let mut buf = [0_u8; 2048];
        for _ in 0..ROUNDS + WARMUP {
            let (n, from) = server.recv_from(&mut buf).await.unwrap();
            server.send_to(&buf[..n], from).await.unwrap();
        }
    });
    let client = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let mut buf = [0_u8; 2048];
    let message = [7_u8; PAYLOAD];
    let mut samples = Vec::with_capacity(ROUNDS);
    for i in 0..ROUNDS + WARMUP {
        let t = Instant::now();
        client.send_to(&message, server_addr).await.unwrap();
        client.recv_from(&mut buf).await.unwrap();
        if i >= WARMUP {
            samples.push(t.elapsed().as_secs_f64() * 1e6);
        }
    }
    task.await.unwrap();
    samples
}

fn quinn_endpoints() -> (quinn::Endpoint, quinn::Endpoint, SocketAddr) {
    let cert = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()]).unwrap();
    let cert_der = cert.cert.der().clone();
    let key = rustls::pki_types::PrivatePkcs8KeyDer::from(cert.key_pair.serialize_der());
    let mut server_config = quinn::ServerConfig::with_single_cert(vec![cert_der.clone()], key.into()).unwrap();
    let transport = Arc::get_mut(&mut server_config.transport).unwrap();
    transport.max_concurrent_uni_streams(0_u8.into());
    let server = quinn::Endpoint::server(server_config, "127.0.0.1:0".parse().unwrap()).unwrap();
    let addr = server.local_addr().unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(cert_der).unwrap();
    let client_config = quinn::ClientConfig::with_root_certificates(Arc::new(roots)).unwrap();
    let mut client = quinn::Endpoint::client("127.0.0.1:0".parse().unwrap()).unwrap();
    client.set_default_client_config(client_config);
    (server, client, addr)
}

async fn quic() {
    let (server, client, addr) = quinn_endpoints();
    // The server echoes datagrams and stream data for one connection.
    let echo = tokio::spawn(async move {
        let incoming = server.accept().await.unwrap();
        let conn = incoming.await.unwrap();
        let for_datagrams = conn.clone();
        let datagrams = tokio::spawn(async move {
            while let Ok(d) = for_datagrams.read_datagram().await {
                if for_datagrams.send_datagram(d).is_err() {
                    break;
                }
            }
        });
        if let Ok((mut send, mut recv)) = conn.accept_bi().await {
            let mut buf = [0_u8; PAYLOAD];
            while recv.read_exact(&mut buf).await.is_ok() {
                if send.write_all(&buf).await.is_err() {
                    break;
                }
            }
        }
        datagrams.abort();
    });

    let t = Instant::now();
    let conn = client.connect(addr, "localhost").unwrap().await.unwrap();
    let handshake_us = t.elapsed().as_secs_f64() * 1e6;
    println!("QUIC handshake (TLS 1.3, loopback)  {handshake_us:.0} us; max datagram size {:?} bytes", conn.max_datagram_size());

    let message = bytes::Bytes::from(vec![7_u8; PAYLOAD]);
    let mut datagram_samples = Vec::with_capacity(ROUNDS);
    for i in 0..ROUNDS + WARMUP {
        let t = Instant::now();
        conn.send_datagram(message.clone()).unwrap();
        conn.read_datagram().await.unwrap();
        if i >= WARMUP {
            datagram_samples.push(t.elapsed().as_secs_f64() * 1e6);
        }
    }
    let stats = conn.stats();
    report("QUIC unreliable datagram", datagram_samples);
    println!(
        "  client sent {} UDP datagrams / {} bytes for {} application datagrams of {PAYLOAD} bytes: {:.1} wire bytes per application datagram (includes the acknowledgements the client sends back)",
        stats.udp_tx.datagrams,
        stats.udp_tx.bytes,
        ROUNDS + WARMUP,
        stats.udp_tx.bytes as f64 / (ROUNDS + WARMUP) as f64
    );

    let (mut send, mut recv) = conn.open_bi().await.unwrap();
    let mut stream_samples = Vec::with_capacity(ROUNDS);
    let mut buf = [0_u8; PAYLOAD];
    for i in 0..ROUNDS + WARMUP {
        let t = Instant::now();
        send.write_all(&[7_u8; PAYLOAD]).await.unwrap();
        recv.read_exact(&mut buf).await.unwrap();
        if i >= WARMUP {
            stream_samples.push(t.elapsed().as_secs_f64() * 1e6);
        }
    }
    report("QUIC reliable stream", stream_samples);
    conn.close(0_u8.into(), b"done");
    let _ = tokio::time::timeout(Duration::from_secs(2), echo).await;
}

/// A UDP proxy between a client and a server that delays every packet by `delay` in each
/// direction and drops a fraction of them (`loss_ppm` parts per million), like a bad link.
async fn lossy_proxy(server: SocketAddr, delay: Duration, loss_ppm: u64, seed: u64) -> SocketAddr {
    use tokio::net::UdpSocket;
    let front = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let back = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let addr = front.local_addr().unwrap();
    let client = Arc::new(std::sync::Mutex::new(None::<SocketAddr>));
    let rng = Arc::new(std::sync::Mutex::new(seed | 1));
    let lost = move |rng: &std::sync::Mutex<u64>| {
        let mut x = rng.lock().unwrap();
        *x ^= *x << 13;
        *x ^= *x >> 7;
        *x ^= *x << 17;
        *x % 1_000_000 < loss_ppm
    };
    {
        let (front, back, client, rng) = (front.clone(), back.clone(), client.clone(), rng.clone());
        tokio::spawn(async move {
            let mut buf = vec![0_u8; 2048];
            loop {
                // On Windows a send to a closed port makes the next receive fail; keep going.
                let Ok((n, from)) = front.recv_from(&mut buf).await else { continue };
                *client.lock().unwrap() = Some(from);
                if lost(&rng) {
                    continue;
                }
                let packet = buf[..n].to_vec();
                let back = back.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(delay).await;
                    let _ = back.send_to(&packet, server).await;
                });
            }
        });
    }
    tokio::spawn(async move {
        let mut buf = vec![0_u8; 2048];
        loop {
            let Ok((n, _)) = back.recv_from(&mut buf).await else { continue };
            if lost(&rng) {
                continue;
            }
            let Some(to) = *client.lock().unwrap() else { continue };
            let packet = buf[..n].to_vec();
            let front = front.clone();
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                let _ = front.send_to(&packet, to).await;
            });
        }
    });
    addr
}

/// 60 messages per second of 200 bytes for `seconds`, each stamped with its send time, over QUIC
/// through a lossy proxy: once as unreliable datagrams and once on one reliable stream. Prints how
/// many arrived and the one-way delay.
async fn quic_under_loss(delay_ms: u64, loss_percent: u64, seconds: u64) {
    let (server, client, server_addr) = quinn_endpoints();
    let proxy = lossy_proxy(server_addr, Duration::from_millis(delay_ms), loss_percent * 10_000, 0x5eed).await;
    let start = Instant::now();
    let datagram_delays = Arc::new(std::sync::Mutex::new(Vec::<f64>::new()));
    let stream_delays = Arc::new(std::sync::Mutex::new(Vec::<f64>::new()));
    let (dd, sd) = (datagram_delays.clone(), stream_delays.clone());
    let receiver = tokio::spawn(async move {
        let conn = server.accept().await.unwrap().await.unwrap();
        let datagrams = conn.clone();
        let reader = tokio::spawn(async move {
            while let Ok(d) = datagrams.read_datagram().await {
                let sent = u64::from_le_bytes(d[..8].try_into().unwrap());
                dd.lock().unwrap().push((start.elapsed().as_nanos() as u64 - sent) as f64 / 1e3);
            }
        });
        if let Ok((_send, mut recv)) = conn.accept_bi().await {
            let mut buf = [0_u8; 200];
            while recv.read_exact(&mut buf).await.is_ok() {
                let sent = u64::from_le_bytes(buf[..8].try_into().unwrap());
                sd.lock().unwrap().push((start.elapsed().as_nanos() as u64 - sent) as f64 / 1e3);
            }
        }
        let _ = reader.await;
    });
    let conn = client.connect(proxy, "localhost").unwrap().await.unwrap();
    let (mut stream, _recv) = conn.open_bi().await.unwrap();
    let total = seconds * 60;
    let mut tick = tokio::time::interval(Duration::from_micros(16_667));
    for _ in 0..total {
        tick.tick().await;
        let mut message = vec![0_u8; 200];
        message[..8].copy_from_slice(&(start.elapsed().as_nanos() as u64).to_le_bytes());
        let _ = conn.send_datagram(bytes::Bytes::from(message.clone()));
        stream.write_all(&message).await.unwrap();
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    conn.close(0_u8.into(), b"done");
    let _ = tokio::time::timeout(Duration::from_secs(2), receiver).await;
    let describe = |name: &str, delays: &Arc<std::sync::Mutex<Vec<f64>>>| {
        let mut v = delays.lock().unwrap().clone();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if v.is_empty() {
            println!("  {name:<20} nothing arrived");
            return;
        }
        let pct = |p: f64| v[((v.len() - 1) as f64 * p) as usize] / 1e3;
        println!(
            "  {name:<20} arrived {:>5} of {total} ({:>5.1} %)   one-way delay ms: p50 {:>6.1}   p99 {:>6.1}   max {:>7.1}",
            v.len(),
            v.len() as f64 * 100.0 / total as f64,
            pct(0.5),
            pct(0.99),
            v[v.len() - 1] / 1e3
        );
    };
    println!("QUIC through a proxy: {delay_ms} ms each way ({} ms round trip), {loss_percent} % loss each way, {total} messages of 200 bytes at 60 per second", delay_ms * 2);
    describe("unreliable datagram", &datagram_delays);
    describe("reliable stream", &stream_delays);
}

fn main() {
    println!("loopback, {PAYLOAD}-byte ping-pong, {ROUNDS} round trips after {WARMUP} warm-up, microseconds (lower is better)");
    report("plain UDP, blocking threads", plain_udp_blocking());
    let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
    report("plain UDP through tokio", runtime.block_on(plain_udp_tokio()));
    runtime.block_on(quic());
    println!();
    runtime.block_on(quic_under_loss(50, 0, 15));
    runtime.block_on(quic_under_loss(50, 5, 15));
    runtime.block_on(quic_under_loss(50, 15, 15));
}
