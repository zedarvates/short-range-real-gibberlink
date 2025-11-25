use criterion::{black_box, criterion_group, criterion_main, Criterion};
use gibberlink_core::protocol::ProtocolEngine;
use gibberlink_core::crypto::CryptoEngine;
use tokio::runtime::Runtime;

fn protocol_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_operations");
    let rt = Runtime::new().unwrap();

    // Handshake initiation benchmark
    group.bench_function("handshake_initiation", |b| {
        b.iter(|| {
            let mut protocol = ProtocolEngine::new();
            let _result = black_box(rt.block_on(async {
                protocol.initiate_handshake().await
            }));
        });
    });

    // QR code generation benchmark (nonce received)
    group.bench_function("qr_generation", |b| {
        b.iter(|| {
            let protocol = ProtocolEngine::new();
            let nonce = CryptoEngine::generate_nonce();

            let _result = black_box(rt.block_on(async {
                protocol.receive_nonce(&nonce).await
            }));
        });
    });

    // QR processing benchmark
    group.bench_function("qr_processing", |b| {
        b.iter(|| {
            let protocol = ProtocolEngine::new();
            let nonce = CryptoEngine::generate_nonce();

            // Pre-generate QR data
            let qr_svg = rt.block_on(async {
                protocol.receive_nonce(&nonce).await.unwrap()
            });

            // Simulate QR data extraction (normally from camera)
            let qr_data = qr_svg.as_bytes()[..500].to_vec();

            let _result = black_box(rt.block_on(async {
                let mut protocol = ProtocolEngine::new();
                protocol.process_qr_payload(&qr_data).await
            }));
        });
    });

    // ACK reception benchmark
    group.bench_function("ack_reception", |b| {
        b.iter(|| {
            let mut protocol = ProtocolEngine::new();
            let nonce = CryptoEngine::generate_nonce();

            // Set up protocol state
            rt.block_on(async {
                protocol.receive_nonce(&nonce).await.unwrap();
                let qr_data = protocol.receive_nonce(&nonce).await.unwrap().as_bytes()[..500].to_vec();
                protocol.process_qr_payload(&qr_data).await.unwrap();
            });

            let _result = black_box(rt.block_on(async {
                protocol.receive_ack().await
            }));
        });
    });

    // Complete handshake flow benchmark
    group.bench_function("complete_handshake_flow", |b| {
        b.iter(|| {
            let _result = black_box(perform_handshake_flow());
        });
    });

    // State transition performance
    group.bench_function("state_transitions", |b| {
        b.iter(|| {
            let mut protocol = ProtocolEngine::new();

            // Simulate state machine transitions
            let nonce = CryptoEngine::generate_nonce();

            rt.block_on(async {
                // Just perform the operations without asserting equality since ProtocolState doesn't implement PartialEq
                let _state = protocol.get_state().await;
                protocol.initiate_handshake().await.unwrap();
                // Note: Actual state transition depends on audio engine
                let _qr = protocol.receive_nonce(&nonce).await.unwrap();
                let qr_data = _qr.as_bytes()[..500].to_vec();
                protocol.process_qr_payload(&qr_data).await.unwrap();
                protocol.receive_ack().await.unwrap();
            });
        });
    });

    group.finish();
}

fn perform_handshake_flow() {
    let rt = Runtime::new().unwrap();

    // Device A (initiator)
    let mut device_a = ProtocolEngine::new();

    // Device B (receiver)
    let device_b = ProtocolEngine::new();

    rt.block_on(async {
        // Step 1: Device A initiates handshake
        device_a.initiate_handshake().await.unwrap();

        // Generate nonce (normally from audio)
        let nonce = CryptoEngine::generate_nonce();

        // Step 2: Device B receives nonce and generates QR
        let qr_svg = device_b.receive_nonce(&nonce).await.unwrap();

        // Step 3: Device A processes QR (simulated scanning)
        let qr_data = qr_svg.as_bytes()[..500].to_vec();
        let mut device_a = device_a;
        device_a.process_qr_payload(&qr_data).await.unwrap();

        // Step 4: Device B receives ACK
        device_b.receive_ack().await.unwrap();
    });
}

fn latency_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_latency");
    let rt = Runtime::new().unwrap();

    // Target: <300ms total handshake time
    group.bench_function("total_handshake_latency", |b| {
        b.iter(|| {
            let _ = black_box(perform_handshake_flow());
        });
    });

    // Target: QR display <10ms
    group.bench_function("qr_display_latency", |b| {
        b.iter(|| {
            let protocol = ProtocolEngine::new();
            let nonce = CryptoEngine::generate_nonce();

            let start = std::time::Instant::now();
            let _qr = rt.block_on(async {
                protocol.receive_nonce(&nonce).await
            });
            let duration = start.elapsed();

            assert!(duration.as_millis() < 10, "QR display took {}ms", duration.as_millis());
        });
    });

    group.finish();
}

fn throughput_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_throughput");
    let _rt = Runtime::new().unwrap();

    // Concurrent handshake throughput
    group.bench_function("concurrent_handshakes_10", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..10).map(|_| {
                std::thread::spawn(|| {
                    perform_handshake_flow()
                })
            }).collect();

            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

criterion_group!(benches, protocol_benchmarks, latency_benchmarks, throughput_benchmarks);
criterion_main!(benches);