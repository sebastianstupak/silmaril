//! TLS performance benchmarks
//!
//! Measures TLS encryption/decryption performance, handshake latency,
//! and throughput under various conditions.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use engine_networking::tls::certificates::generate_and_save_self_signed_cert;
use engine_networking::tls::{
    CertificateVerification, SelfSignedConfig, TlsClientConfigBuilder, TlsClientConnection,
    TlsServer, TlsServerConfigBuilder,
};
use std::env;
use std::path::PathBuf;
use tokio::runtime::Runtime;

/// Setup test certificates for benchmarks
fn setup_bench_certs() -> (PathBuf, PathBuf) {
    let temp_dir = env::temp_dir();
    let cert_path = temp_dir.join("bench_cert.pem");
    let key_path = temp_dir.join("bench_key.pem");

    if !cert_path.exists() {
        let config = SelfSignedConfig::new("localhost").add_san("127.0.0.1").validity_days(365);

        generate_and_save_self_signed_cert(&config, &cert_path, &key_path)
            .expect("Failed to generate benchmark certificate");
    }

    (cert_path, key_path)
}

/// Benchmark TLS handshake latency (cold start)
fn bench_handshake_cold(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (cert_path, key_path) = setup_bench_certs();

    c.bench_function("tls_handshake_cold", |b| {
        b.to_async(&rt).iter(|| async {
            let server_config = TlsServerConfigBuilder::new()
                .certificate(&cert_path, &key_path)
                .build()
                .expect("Failed to build server config");

            let server = TlsServer::bind("127.0.0.1:0", server_config)
                .await
                .expect("Failed to bind server");
            let server_addr = server.local_addr().expect("Failed to get server address");

            // Spawn server
            let server_handle = tokio::spawn(async move {
                server.accept().await.ok();
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Client handshake (this is what we're measuring)
            let client_config = TlsClientConfigBuilder::new()
                .verification(CertificateVerification::Disabled)
                .build()
                .expect("Failed to build client config");

            let start = std::time::Instant::now();
            let client =
                TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
                    .await;
            let elapsed = start.elapsed();

            server_handle.await.ok();
            client.ok();

            black_box(elapsed)
        });
    });
}

/// Benchmark message encryption throughput
fn bench_encryption_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (cert_path, key_path) = setup_bench_certs();

    let mut group = c.benchmark_group("tls_encryption_throughput");

    for size in [1024, 4096, 16384, 65536, 262144, 1048576].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let server_config = TlsServerConfigBuilder::new()
                    .certificate(&cert_path, &key_path)
                    .build()
                    .expect("Failed to build server config");

                let server = TlsServer::bind("127.0.0.1:0", server_config)
                    .await
                    .expect("Failed to bind server");
                let server_addr = server.local_addr().expect("Failed to get server address");

                tokio::spawn(async move {
                    let mut conn = server.accept().await.expect("Failed to accept");
                    conn.recv().await.ok();
                });

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let client_config = TlsClientConfigBuilder::new()
                    .verification(CertificateVerification::Disabled)
                    .build()
                    .expect("Failed to build client config");

                let mut client = TlsClientConnection::connect(
                    server_addr.to_string(),
                    "localhost",
                    client_config,
                )
                .await
                .expect("Failed to connect");

                let data = vec![0u8; size];
                let start = std::time::Instant::now();
                client.send(&data).await.expect("Failed to send");
                let elapsed = start.elapsed();

                black_box(elapsed)
            });
        });
    }

    group.finish();
}

/// Benchmark message decryption throughput
fn bench_decryption_throughput(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (cert_path, key_path) = setup_bench_certs();

    let mut group = c.benchmark_group("tls_decryption_throughput");

    for size in [1024, 4096, 16384, 65536, 262144, 1048576].iter() {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.to_async(&rt).iter(|| async {
                let server_config = TlsServerConfigBuilder::new()
                    .certificate(&cert_path, &key_path)
                    .build()
                    .expect("Failed to build server config");

                let server = TlsServer::bind("127.0.0.1:0", server_config)
                    .await
                    .expect("Failed to bind server");
                let server_addr = server.local_addr().expect("Failed to get server address");

                let data = vec![0u8; size];
                let data_clone = data.clone();

                tokio::spawn(async move {
                    let mut conn = server.accept().await.expect("Failed to accept");
                    conn.send(&data_clone).await.ok();
                });

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let client_config = TlsClientConfigBuilder::new()
                    .verification(CertificateVerification::Disabled)
                    .build()
                    .expect("Failed to build client config");

                let mut client = TlsClientConnection::connect(
                    server_addr.to_string(),
                    "localhost",
                    client_config,
                )
                .await
                .expect("Failed to connect");

                let start = std::time::Instant::now();
                let received = client.recv().await.expect("Failed to recv");
                let elapsed = start.elapsed();

                assert_eq!(received.len(), size);
                black_box(elapsed)
            });
        });
    }

    group.finish();
}

/// Benchmark concurrent connections
fn bench_concurrent_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (cert_path, key_path) = setup_bench_certs();

    let mut group = c.benchmark_group("tls_concurrent_connections");

    for count in [1, 10, 100].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, &count| {
            b.to_async(&rt).iter(|| async {
                let server_config = TlsServerConfigBuilder::new()
                    .certificate(&cert_path, &key_path)
                    .build()
                    .expect("Failed to build server config");

                let server = TlsServer::bind("127.0.0.1:0", server_config)
                    .await
                    .expect("Failed to bind server");
                let server_addr = server.local_addr().expect("Failed to get server address");

                // Spawn server handling multiple connections
                tokio::spawn(async move {
                    for _ in 0..count {
                        let mut conn = server.accept().await.expect("Failed to accept");
                        tokio::spawn(async move {
                            conn.recv().await.ok();
                        });
                    }
                });

                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                let start = std::time::Instant::now();

                // Create concurrent connections
                let mut handles = vec![];
                for _ in 0..count {
                    let addr = server_addr.to_string();
                    let handle = tokio::spawn(async move {
                        let client_config = TlsClientConfigBuilder::new()
                            .verification(CertificateVerification::Disabled)
                            .build()
                            .expect("Failed to build client config");

                        let mut client =
                            TlsClientConnection::connect(addr, "localhost", client_config)
                                .await
                                .expect("Failed to connect");

                        client.send(b"test").await.ok();
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.ok();
                }

                let elapsed = start.elapsed();
                black_box(elapsed)
            });
        });
    }

    group.finish();
}

/// Benchmark round-trip latency
fn bench_round_trip_latency(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (cert_path, key_path) = setup_bench_certs();

    c.bench_function("tls_round_trip_latency", |b| {
        b.to_async(&rt).iter(|| async {
            let server_config = TlsServerConfigBuilder::new()
                .certificate(&cert_path, &key_path)
                .build()
                .expect("Failed to build server config");

            let server = TlsServer::bind("127.0.0.1:0", server_config)
                .await
                .expect("Failed to bind server");
            let server_addr = server.local_addr().expect("Failed to get server address");

            tokio::spawn(async move {
                let mut conn = server.accept().await.expect("Failed to accept");
                let msg = conn.recv().await.expect("Failed to recv");
                conn.send(&msg).await.expect("Failed to send");
            });

            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let client_config = TlsClientConfigBuilder::new()
                .verification(CertificateVerification::Disabled)
                .build()
                .expect("Failed to build client config");

            let mut client =
                TlsClientConnection::connect(server_addr.to_string(), "localhost", client_config)
                    .await
                    .expect("Failed to connect");

            let message = b"ping";
            let start = std::time::Instant::now();
            client.send(message).await.expect("Failed to send");
            let response = client.recv().await.expect("Failed to recv");
            let elapsed = start.elapsed();

            assert_eq!(response, message);
            black_box(elapsed)
        });
    });
}

criterion_group!(
    benches,
    bench_handshake_cold,
    bench_encryption_throughput,
    bench_decryption_throughput,
    bench_concurrent_connections,
    bench_round_trip_latency,
);
criterion_main!(benches);
