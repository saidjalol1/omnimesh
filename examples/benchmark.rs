use omnimesh::buffer::PayloadStorage;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::transport::config::TransportConfig;
use omnimesh::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use omnimesh::runtime::transport::tcp::TcpTransport;
use omnimesh::runtime::RoutingTable;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn create_benchmark_message(
    sender: Did,
    recipient: Did,
    msg_id: [u8; 16],
    timestamp: u64,
) -> SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY> {
    let header = EnvelopeHeader::new(
        1,
        MessageId::new(msg_id),
        sender,
        recipient,
        0,
        timestamp,
        Priority::Critical,
        PayloadType::Raw,
    );
    SignedEnvelope {
        header,
        payload: PayloadStorage::new(),
        signature: [0u8; 64],
    }
}

fn current_time_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

fn main() {
    println!("🚀 Starting OMNI-MESH Microsecond Latency Benchmark...");
    let iterations = 10_000;

    let node_a_did = Did::new([0x0A; 32]);
    let node_b_did = Did::new([0x0B; 32]);

    // Setup Server (Node A)
    let a_config = TransportConfig::new(
        "127.0.0.1:9050".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let a_routing = Arc::new(RoutingTable::new());
    a_routing.update_route(node_b_did, "127.0.0.1:9060".parse().unwrap());
    let a_transport = TcpTransport::new(a_config, a_routing).unwrap();

    // Setup Client (Node B)
    let b_config = TransportConfig::new(
        "127.0.0.1:9060".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let b_routing = Arc::new(RoutingTable::new());
    b_routing.update_route(node_a_did, "127.0.0.1:9050".parse().unwrap());
    let b_transport = TcpTransport::new(b_config, b_routing).unwrap();

    thread::sleep(Duration::from_millis(200)); // wait for binds

    // Node A simply echoes everything back
    let a_thread = thread::spawn(move || {
        let mut count = 0;
        loop {
            if let Some(envelope) = a_transport.receive() {
                // Echo back to sender
                let reply = create_benchmark_message(
                    node_a_did,
                    envelope.header.sender_did,
                    envelope.header.message_id.0,
                    envelope.header.timestamp_us, // Pass back original timestamp
                );
                a_transport.send(&reply).unwrap();
                count += 1;
                if count >= iterations { break; }
            }
        }
    });

    println!("Performing {} Round-Trip TCP Exchanges...", iterations);
    let mut total_rtt_us = 0;
    let mut max_rtt_us = 0;
    let mut min_rtt_us = u64::MAX;

    // Node B blasts messages and measures RTT
    for i in 0..iterations {
        let start_time = current_time_us();
        
        let mut msg_id = [0u8; 16];
        msg_id[0] = (i % 256) as u8;

        let envelope = create_benchmark_message(node_b_did, node_a_did, msg_id, start_time);
        b_transport.send(&envelope).unwrap();

        // Block until reply
        loop {
            if let Some(reply) = b_transport.receive() {
                let end_time = current_time_us();
                let rtt = end_time - reply.header.timestamp_us;
                
                total_rtt_us += rtt;
                if rtt > max_rtt_us { max_rtt_us = rtt; }
                if rtt < min_rtt_us { min_rtt_us = rtt; }
                break;
            }
        }
    }

    a_thread.join().unwrap();

    let avg_rtt_us = total_rtt_us as f64 / iterations as f64;
    let one_way_latency = avg_rtt_us / 2.0;

    println!("\n📊 BENCHMARK RESULTS:");
    println!("------------------------------------------------");
    println!("Messages Exchanged : {} (Full Round Trips)", iterations);
    println!("Average RTT        : {:.2} μs (microseconds)", avg_rtt_us);
    println!("Min RTT            : {} μs", min_rtt_us);
    println!("Max RTT            : {} μs", max_rtt_us);
    println!("------------------------------------------------");
    println!("🚀 OMNI-MESH ONE-WAY LATENCY: {:.2} μs", one_way_latency);
    println!("------------------------------------------------");
}
