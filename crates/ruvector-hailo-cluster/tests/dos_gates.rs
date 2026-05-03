//! End-to-end test for the iter-180 gRPC byte-cap DoS gate
//! (`max_decoding_message_size`).
//!
//! Stands up an `EmbeddingServer` with a deliberately tight 4 KB cap,
//! sends an 8 KB embed text, and asserts the server rejects with
//! `Code::OutOfRange` and the error string mentions the limit. Locks
//! in iter-180 (and by extension iter-190's encoding cap, iter-181/
//! 182/183/184/192's parity work) so a future change that drops the
//! cap doesn't regress unnoticed.
//!
//! Mirrors the in-process mock pattern from `rate_limit_interceptor.rs`
//! and `tls_roundtrip.rs` — no NPU dependency, no fakeworker
//! subprocess, runs on x86 dev hosts and aarch64 Pi alike.

use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Code, Request, Response, Status};

use ruvector_hailo_cluster::proto::embedding_client::EmbeddingClient;
use ruvector_hailo_cluster::proto::embedding_server::{Embedding, EmbeddingServer};
use ruvector_hailo_cluster::proto::{
    EmbedBatchRequest, EmbedRequest, EmbedResponse, EmbedStreamResponse, HealthRequest,
    HealthResponse, StatsRequest, StatsResponse,
};

#[derive(Default, Clone)]
struct EchoMockWorker;

#[tonic::async_trait]
impl Embedding for EchoMockWorker {
    async fn embed(
        &self,
        _request: Request<EmbedRequest>,
    ) -> Result<Response<EmbedResponse>, Status> {
        // Should never reach the handler — the byte-cap rejects before
        // dispatch — but if it does, we want the test to fail loudly
        // rather than silently succeed.
        Ok(Response::new(EmbedResponse {
            vector: vec![0.0; 384],
            dim: 384,
            latency_us: 0,
        }))
    }

    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        Ok(Response::new(HealthResponse {
            version: "dos-mock".into(),
            device_id: "dos:0".into(),
            model_fingerprint: "fp:dos".into(),
            ready: true,
            npu_temp_ts0_celsius: 0.0,
            npu_temp_ts1_celsius: 0.0,
        }))
    }

    type EmbedStreamStream = Pin<
        Box<dyn futures_core::Stream<Item = Result<EmbedStreamResponse, Status>> + Send + 'static>,
    >;

    async fn embed_stream(
        &self,
        _request: Request<EmbedBatchRequest>,
    ) -> Result<Response<Self::EmbedStreamStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<EmbedStreamResponse, Status>>(1);
        drop(tx);
        Ok(Response::new(Box::pin(
            tokio_stream::wrappers::ReceiverStream::new(rx),
        )))
    }

    async fn get_stats(
        &self,
        _request: Request<StatsRequest>,
    ) -> Result<Response<StatsResponse>, Status> {
        Ok(Response::new(StatsResponse::default()))
    }
}

/// Stand up an EmbeddingServer with `max_decoding_message_size = cap_bytes`.
/// Returns the bound `SocketAddr` once the listener is accepting.
async fn start_capped_server(cap_bytes: usize) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let incoming = TcpListenerStream::new(listener);

    let svc = EmbeddingServer::new(EchoMockWorker).max_decoding_message_size(cap_bytes);
    tokio::spawn(async move {
        Server::builder()
            .add_service(svc)
            .serve_with_incoming(incoming)
            .await
            .ok();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    addr
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embed_request_above_decoding_cap_returns_out_of_range() {
    // Cap chosen deliberately small so a tiny test payload trips it.
    // Same code path as iter-180's 64 KB production cap; only the
    // numeric value differs.
    let cap = 4 * 1024;
    let addr = start_capped_server(cap).await;
    let endpoint = tonic::transport::Endpoint::from_shared(format!("http://{}", addr))
        .unwrap()
        .connect_timeout(Duration::from_secs(2));
    let channel = endpoint.connect().await.expect("connect");
    let mut client = EmbeddingClient::new(channel);

    // Build a payload > cap. 8 KB is comfortably over the 4 KB cap
    // even after prost framing strips a few bytes.
    let oversized: String = "x".repeat(8 * 1024);
    let req = tonic::Request::new(EmbedRequest {
        text: oversized,
        max_seq: 128,
        request_id: "dos-gates-test".into(),
    });

    let err = client
        .embed(req)
        .await
        .expect_err("oversized embed must be rejected by the byte cap");

    assert_eq!(
        err.code(),
        Code::OutOfRange,
        "byte-cap rejection should surface as OutOfRange (status code {:?}); \
         got {:?} with message {:?}",
        Code::OutOfRange,
        err.code(),
        err.message(),
    );
    let msg = err.message();
    assert!(
        msg.contains("decoded message length too large") || msg.contains(&cap.to_string()),
        "OutOfRange status should mention the limit; got message {:?}",
        msg
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn embed_request_below_decoding_cap_succeeds() {
    // Companion to the rejection test: an under-cap payload sails
    // through, proving the cap isn't blocking legitimate traffic.
    // Same cap = 4 KB, payload = 1 KB.
    let cap = 4 * 1024;
    let addr = start_capped_server(cap).await;
    let endpoint = tonic::transport::Endpoint::from_shared(format!("http://{}", addr))
        .unwrap()
        .connect_timeout(Duration::from_secs(2));
    let channel = endpoint.connect().await.expect("connect");
    let mut client = EmbeddingClient::new(channel);

    let small: String = "x".repeat(1024);
    let req = tonic::Request::new(EmbedRequest {
        text: small,
        max_seq: 128,
        request_id: "dos-gates-test-ok".into(),
    });

    let resp = client.embed(req).await.expect("under-cap embed should succeed");
    let body = resp.into_inner();
    assert_eq!(body.dim, 384, "echo mock returns dim=384");
}
