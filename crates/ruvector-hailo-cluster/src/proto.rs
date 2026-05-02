//! Generated tonic stubs for `proto/embedding.proto`.
//!
//! The proto package is `ruvector.hailo.v1`, so tonic-build emits a
//! `ruvector.hailo.v1` Rust module containing the message structs +
//! the `embedding_client` / `embedding_server` modules.

#![allow(clippy::pedantic)]
#![allow(missing_docs)]

tonic::include_proto!("ruvector.hailo.v1");

/// Canonical metadata header for cross-call request correlation.
/// W3C-style — workers and any intermediary tracing infra can grep
/// `x-request-id` without knowing the proto schema.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// 24-hex-char correlation ID with a sortable timestamp prefix.
///
/// Layout: `<16-hex-ms-since-epoch><8-hex-rand>` — 24 chars total.
/// Lexically-sorted IDs match chronological order, so log queries
/// like `grep request_id | sort | uniq` reveal call sequence without
/// timestamp alignment. Random suffix has 32 bits of entropy from
/// xorshift64* — collisions only matter within a single ms.
///
/// Public so callers (web handlers, batch ingest CLIs, custom
/// transports) can generate matching IDs without going through
/// `GrpcTransport`. Not crypto-grade.
pub fn random_request_id() -> String {
    use std::cell::Cell;
    thread_local! {
        static STATE: Cell<u64> = const { Cell::new(0) };
    }
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let rand32 = STATE.with(|s| {
        let mut x = s.get();
        if x == 0 {
            x = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64).unwrap_or(1);
            if x == 0 { x = 0x9E3779B97F4A7C15; }
        }
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        s.set(x);
        // Top 32 bits — they cycle through PRNG state most uniformly.
        (x >> 32) as u32
    });
    format!("{:016x}{:08x}", now_ms, rand32)
}

/// Inject `request_id` as the `x-request-id` gRPC metadata header on
/// an outgoing tonic request. Best-effort: invalid characters silently
/// fall through (the proto field carries the same value as fallback).
pub fn inject_request_id<T>(req: &mut tonic::Request<T>, request_id: &str) {
    if let Ok(v) = request_id.parse::<tonic::metadata::MetadataValue<_>>() {
        req.metadata_mut().insert(REQUEST_ID_HEADER, v);
    }
}

/// Pull `request_id` out of an incoming tonic request's metadata,
/// falling back to a `proto_field` if the header is absent. Returns
/// owned `String` to decouple from the request lifetime.
pub fn extract_request_id<T>(req: &tonic::Request<T>, proto_field: &str) -> String {
    if let Some(v) = req.metadata().get(REQUEST_ID_HEADER) {
        if let Ok(s) = v.to_str() {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    proto_field.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_message_types_are_constructible() {
        let req = EmbedRequest {
            text: "hello world".into(),
            max_seq: 128,
            request_id: "abc123".into(),
        };
        assert_eq!(req.text, "hello world");
        assert_eq!(req.max_seq, 128);
        assert_eq!(req.request_id, "abc123");

        let resp = EmbedResponse {
            vector: vec![0.1, 0.2, 0.3],
            dim: 3,
            latency_us: 1234,
        };
        assert_eq!(resp.vector.len(), 3);
        assert_eq!(resp.dim, 3);

        let health = HealthResponse {
            version: "ruvector-hailo 0.1.0".into(),
            device_id: "0001:01:00.0".into(),
            model_fingerprint: "sha256:abc".into(),
            ready: true,
        };
        assert!(health.ready);
        assert_eq!(health.device_id, "0001:01:00.0");
    }

    #[test]
    fn generated_messages_roundtrip_protobuf() {
        use prost::Message;

        let req = EmbedRequest {
            text: "ruvector".into(),
            max_seq: 64,
            request_id: "deadbeef".into(),
        };
        let buf = req.encode_to_vec();
        let decoded = EmbedRequest::decode(&buf[..]).expect("protobuf roundtrip");
        assert_eq!(decoded, req);
    }

    #[test]
    fn inject_and_extract_request_id_roundtrips_via_metadata() {
        let mut req = tonic::Request::new(EmbedRequest::default());
        inject_request_id(&mut req, "abc-123");
        let extracted = extract_request_id(&req, "");
        assert_eq!(extracted, "abc-123");
    }

    #[test]
    fn extract_request_id_falls_back_to_proto_field_when_header_absent() {
        let req = tonic::Request::new(EmbedRequest::default());
        let extracted = extract_request_id(&req, "from-proto");
        assert_eq!(extracted, "from-proto");
    }

    #[test]
    fn extract_request_id_prefers_metadata_over_proto_field() {
        let mut req = tonic::Request::new(EmbedRequest::default());
        inject_request_id(&mut req, "from-meta");
        let extracted = extract_request_id(&req, "from-proto");
        assert_eq!(extracted, "from-meta", "metadata wins when both present");
    }

    #[test]
    fn inject_request_id_with_invalid_chars_is_silently_dropped() {
        // Newlines aren't valid in HTTP header values; injection is a
        // no-op and extract falls back to the proto field.
        let mut req = tonic::Request::new(EmbedRequest::default());
        inject_request_id(&mut req, "bad\nid");
        let extracted = extract_request_id(&req, "fallback");
        assert_eq!(extracted, "fallback", "invalid header value should be silently ignored");
    }

    #[test]
    fn random_request_id_has_24_hex_chars() {
        let id = random_request_id();
        assert_eq!(id.len(), 24, "expected 24-char id, got {:?}", id);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn random_request_id_two_consecutive_ids_sort_chronologically() {
        // Generate id1, sleep so the ms timestamp definitely advances,
        // generate id2. id1 should sort before id2 lexically.
        let id1 = random_request_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let id2 = random_request_id();
        assert!(id1 < id2, "expected id1 < id2, got id1={:?} id2={:?}", id1, id2);
        assert_eq!(id1.len(), id2.len(), "format stability");
    }

    #[test]
    fn random_request_id_uniqueness_within_same_ms() {
        let mut ids = std::collections::HashSet::new();
        for _ in 0..100 {
            ids.insert(random_request_id());
        }
        assert_eq!(ids.len(), 100, "duplicate IDs in 100 rapid calls");
    }

    #[test]
    fn random_request_id_prefix_decodes_to_recent_ms() {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let id = random_request_id();
        let prefix_ms = u64::from_str_radix(&id[..16], 16)
            .expect("prefix should be parseable hex u64");
        let delta = prefix_ms.abs_diff(now_ms);
        assert!(delta < 5_000,
            "prefix ms {} differs from now {} by {}ms", prefix_ms, now_ms, delta);
    }

    #[test]
    fn generated_client_and_server_modules_exist() {
        // Compile-time checks — if these fail, tonic-build didn't emit the
        // expected modules. Just touching the type names is enough.
        fn _t<T>() {}
        _t::<embedding_client::EmbeddingClient<tonic::transport::Channel>>();
        // Server side is a generic over an `Embedding` trait impl; just
        // referencing the trait keeps the test trivially compile-only.
        fn _s<T: embedding_server::Embedding>() {}
    }
}
