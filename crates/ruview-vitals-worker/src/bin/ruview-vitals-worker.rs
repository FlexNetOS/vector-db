//! `ruview-vitals-worker` — per-Pi WiFi-CSI vital signs worker
//! (ADR-183 Tier 1, iter 1 scaffold).
//!
//! This iter brings up the UDP listener, parses ADR-018 frames, and
//! logs a one-line summary per packet. Sliding window, vitals
//! pipeline, brain POST shim, and gRPC service land in subsequent
//! /loop iterations per the ADR.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::net::UdpSocket;
use tracing_subscriber::EnvFilter;

use ruview_vitals_worker::{Adr018Frame, Adr018Header, Config, Result, VERSION};

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<()> {
    init_tracing();
    let cfg = Config::from_env()?;

    tracing::info!(
        version = VERSION,
        node = %cfg.node_name,
        udp = %cfg.udp_listen,
        grpc = %cfg.grpc_listen,
        brain = %cfg.brain_url,
        window_frames = cfg.window_frames,
        "ruview-vitals-worker starting"
    );

    let socket = UdpSocket::bind(cfg.udp_listen).await?;
    tracing::info!(addr = %socket.local_addr()?, "UDP listener up");

    let stats = Arc::new(Counters::default());

    // Periodic stats logger — once per minute. Gives operators a
    // heartbeat without spamming when verbose=false.
    let stats_logger = Arc::clone(&stats);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(60));
        // Skip the first immediate tick — we just logged "starting".
        tick.tick().await;
        loop {
            tick.tick().await;
            let recv = stats_logger.received.load(Ordering::Relaxed);
            let drop = stats_logger.dropped.load(Ordering::Relaxed);
            let frames = stats_logger.frames.load(Ordering::Relaxed);
            tracing::info!(
                packets_received = recv,
                packets_dropped = drop,
                frames_parsed = frames,
                "vitals-worker heartbeat"
            );
        }
    });

    // UDP ingress hot loop. Sized for an MTU-sized datagram + headroom
    // for the largest ESP32-S3 frame (~ 56 subcarriers × 2 × 4
    // antennas + 20 byte header ≈ 468 bytes — 64 KiB is comfortable).
    let mut buf = vec![0u8; 65_536];
    loop {
        let (len, peer) = match socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, "UDP recv_from failed");
                continue;
            }
        };
        stats.received.fetch_add(1, Ordering::Relaxed);

        let datagram = &buf[..len];
        match Adr018Frame::parse(datagram) {
            Some(frame) => {
                stats.frames.fetch_add(1, Ordering::Relaxed);
                if cfg.verbose {
                    log_frame(&peer, &frame.header, len);
                }
                // TODO(adr-183 iter 2): push frame into the sliding
                // window and run the vitals pipeline. For now we just
                // count it.
                let _ = frame;
            }
            None => {
                stats.dropped.fetch_add(1, Ordering::Relaxed);
                // Header-only parse fallback so we still log "what
                // came in" when the payload is short or the magic is
                // off. Useful when bringing up the ESP32 firmware.
                if let Some(hdr) = Adr018Header::parse(datagram) {
                    tracing::warn!(
                        peer = %peer,
                        len,
                        node_id = hdr.node_id,
                        n_subcarriers = hdr.n_subcarriers,
                        n_antennas = hdr.n_antennas,
                        "drop: payload too short for header"
                    );
                } else {
                    tracing::warn!(peer = %peer, len, "drop: not an ADR-018 frame");
                }
            }
        }
    }
}

fn log_frame(peer: &std::net::SocketAddr, hdr: &Adr018Header, len: usize) {
    tracing::debug!(
        peer = %peer,
        len,
        magic = format_args!("0x{:08x}", hdr.magic),
        node_id = hdr.node_id,
        antennas = hdr.n_antennas,
        subcarriers = hdr.n_subcarriers,
        channel = hdr.channel,
        rssi_dbm = hdr.rssi,
        noise_dbm = hdr.noise_floor,
        ts_us = hdr.timestamp_us,
        "ADR-018 frame"
    );
}

fn init_tracing() {
    let filter = EnvFilter::try_from_env("RUVIEW_VITALS_LOG")
        .or_else(|_| EnvFilter::try_new("info,ruview_vitals_worker=info"))
        .expect("default tracing filter");
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(std::io::stderr)
        .init();
}

#[derive(Debug, Default)]
struct Counters {
    received: AtomicU64,
    dropped: AtomicU64,
    frames: AtomicU64,
}
