use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use ed25519_dalek::SigningKey;
use omnimesh::buffer::PayloadStorage;
use omnimesh::config::OmnimeshMode;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use omnimesh::payload;
use omnimesh::runtime::delivery::DeliveryLayer;
use omnimesh::runtime::security::SecurityLayer;
use omnimesh::runtime::storage::StorageLayer;
use rand_core::OsRng;

fn create_test_envelope<const N: usize>(seq: u64, signing_key: &SigningKey) -> SignedEnvelope<N> {
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());
    let recipient_did = Did([0xBB; 32]);

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([seq as u8; 16]),
        sender_did,
        recipient_did,
        sequence_number: seq,
        timestamp_us: 1234567890 + seq,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<N>::new();
    let msg = format!("Benchmark message {}", seq);
    payload_buf.push_bytes(msg.as_bytes()).unwrap();

    SignedEnvelope::sign(header, payload_buf, signing_key)
}

fn bench_envelope_serialization(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let envelope = create_test_envelope::<1024>(0, &signing_key);

    let mut group = c.benchmark_group("envelope_serialization");
    group.throughput(Throughput::Elements(1));

    group.bench_function("serialize_1kb", |b| {
        b.iter(|| {
            let mut buf = [0u8; 2048];
            black_box(envelope.serialize_into(&mut buf))
        });
    });

    group.finish();
}

fn bench_envelope_deserialization(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let envelope = create_test_envelope::<1024>(0, &signing_key);

    let mut buf = [0u8; 2048];
    let len = envelope.serialize_into(&mut buf).unwrap();
    let serialized = &buf[..len];

    let mut group = c.benchmark_group("envelope_deserialization");
    group.throughput(Throughput::Elements(1));

    group.bench_function("deserialize_1kb", |b| {
        b.iter(|| black_box(SignedEnvelope::<1024>::deserialize(serialized)));
    });

    group.finish();
}

fn bench_signature_generation(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let sender_did = Did::new(signing_key.verifying_key().to_bytes());

    let header = EnvelopeHeader {
        version: 7,
        message_id: MessageId([1u8; 16]),
        sender_did,
        recipient_did: Did([0xBB; 32]),
        sequence_number: 0,
        timestamp_us: 1234567890,
        priority: Priority::Normal,
        payload_type: PayloadType::Raw,
    };

    let mut payload_buf = PayloadStorage::<1024>::new();
    payload_buf.push_bytes(b"Benchmark payload").unwrap();

    let mut group = c.benchmark_group("signature");
    group.throughput(Throughput::Elements(1));

    group.bench_function("sign_envelope", |b| {
        b.iter(|| {
            black_box(SignedEnvelope::sign(
                header,
                payload_buf.clone(),
                &signing_key,
            ))
        });
    });

    group.finish();
}

fn bench_signature_verification(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let envelope = create_test_envelope::<1024>(0, &signing_key);
    let mode = OmnimeshMode::production();
    let security = SecurityLayer::new(&mode, None);

    let mut group = c.benchmark_group("signature");
    group.throughput(Throughput::Elements(1));

    group.bench_function("verify_envelope", |b| {
        b.iter(|| black_box(security.verify(&envelope)));
    });

    group.finish();
}

fn bench_delivery_layer(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mode = OmnimeshMode::production();
    let delivery = DeliveryLayer::new(&mode);

    let mut group = c.benchmark_group("delivery");
    group.throughput(Throughput::Elements(1));

    group.bench_function("deliver_new_message", |b| {
        let mut seq = 0u64;
        b.iter(|| {
            let envelope = create_test_envelope::<1024>(seq, &signing_key);
            seq += 1;
            black_box(delivery.deliver(&envelope))
        });
    });

    group.bench_function("deliver_duplicate", |b| {
        let envelope = create_test_envelope::<1024>(999999, &signing_key);
        delivery.deliver(&envelope).unwrap(); // First delivery

        b.iter(|| black_box(delivery.deliver(&envelope)));
    });

    group.finish();
}

fn bench_storage_layer(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mode = OmnimeshMode::production();

    let mut group = c.benchmark_group("storage");
    group.throughput(Throughput::Elements(1));

    group.bench_function("store_message", |b| {
        let mut storage = StorageLayer::new(&mode);
        let mut seq = 0u64;

        b.iter(|| {
            let envelope = create_test_envelope::<1024>(seq, &signing_key);
            seq += 1;
            black_box(storage.store(envelope))
        });
    });

    group.finish();
}

fn bench_payload_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("payload_encoding");
    group.throughput(Throughput::Elements(1));

    group.bench_function("encode_motion_command", |b| {
        let motion = payload::motion_command(1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 100_000);
        b.iter(|| black_box(payload::encode_payload(&motion)));
    });

    group.bench_function("encode_agent_command", |b| {
        let cmd = payload::agent_command("ship", &[0xAA; 32], b"order-123");
        b.iter(|| black_box(payload::encode_payload(&cmd)));
    });

    group.bench_function("encode_heartbeat", |b| {
        let hb = payload::heartbeat(&[0xCC; 32], 60000, 45, 2048, 999);
        b.iter(|| black_box(payload::encode_payload(&hb)));
    });

    group.finish();
}

fn bench_payload_decoding(c: &mut Criterion) {
    let motion = payload::motion_command(1.0, 0.0, 0.0, 0.0, 0.0, 0.5, 100_000);
    let motion_bytes = payload::encode_payload(&motion);

    let cmd = payload::agent_command("ship", &[0xAA; 32], b"order-123");
    let cmd_bytes = payload::encode_payload(&cmd);

    let hb = payload::heartbeat(&[0xCC; 32], 60000, 45, 2048, 999);
    let hb_bytes = payload::encode_payload(&hb);

    let mut group = c.benchmark_group("payload_decoding");
    group.throughput(Throughput::Elements(1));

    group.bench_function("decode_motion_command", |b| {
        b.iter(|| black_box(payload::decode_payload(&motion_bytes)));
    });

    group.bench_function("decode_agent_command", |b| {
        b.iter(|| black_box(payload::decode_payload(&cmd_bytes)));
    });

    group.bench_function("decode_heartbeat", |b| {
        b.iter(|| black_box(payload::decode_payload(&hb_bytes)));
    });

    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let mode = OmnimeshMode::production();
    let security = SecurityLayer::new(&mode, None);
    let delivery = DeliveryLayer::new(&mode);
    let mut storage = StorageLayer::new(&mode);

    let mut group = c.benchmark_group("end_to_end");
    group.throughput(Throughput::Elements(1));

    group.bench_function("sign_verify_deliver_store", |b| {
        let mut seq = 0u64;

        b.iter(|| {
            // 1. Create and sign envelope
            let envelope = create_test_envelope::<1024>(seq, &signing_key);
            seq += 1;

            // 2. Verify signature
            let _ = security.verify(&envelope);

            // 3. Deliver (deduplication)
            let _ = delivery.deliver(&envelope);

            // 4. Store
            let _ = storage.store(envelope);

            black_box(())
        });
    });

    group.finish();
}

fn bench_payload_sizes(c: &mut Criterion) {
    let signing_key = SigningKey::generate(&mut OsRng);

    let mut group = c.benchmark_group("payload_sizes");

    for size in [128, 256, 512, 1024].iter() {
        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            let payload_data = vec![0xAA; size];

            b.iter(|| {
                let sender_did = Did::new(signing_key.verifying_key().to_bytes());
                let header = EnvelopeHeader {
                    version: 7,
                    message_id: MessageId([1u8; 16]),
                    sender_did,
                    recipient_did: Did([0xBB; 32]),
                    sequence_number: 0,
                    timestamp_us: 1234567890,
                    priority: Priority::Normal,
                    payload_type: PayloadType::Raw,
                };

                let mut payload_buf = PayloadStorage::<1024>::new();
                payload_buf
                    .push_bytes(&payload_data[..size.min(1024)])
                    .unwrap();

                black_box(SignedEnvelope::sign(header, payload_buf, &signing_key))
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_envelope_serialization,
    bench_envelope_deserialization,
    bench_signature_generation,
    bench_signature_verification,
    bench_delivery_layer,
    bench_storage_layer,
    bench_payload_encoding,
    bench_payload_decoding,
    bench_end_to_end,
    bench_payload_sizes
);

criterion_main!(benches);
