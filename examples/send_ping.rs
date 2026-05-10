use omnimesh::config::loader::ConfigFile;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use omnimesh::buffer::PayloadStorage;
use omnimesh::runtime::transport::config::TransportConfig;
use omnimesh::runtime::transport::tcp::TcpTransport;
use omnimesh::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use omnimesh::runtime::RoutingTable;
use std::sync::Arc;

fn main() {
    println!("Creating client transport...");
    
    // We bind to an ephemeral port (0) and connect to the daemon (9000)
    let config = TransportConfig::new(
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:9000".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    
    let routing = Arc::new(RoutingTable::new());
    let transport = TcpTransport::new(config, routing).unwrap();
    
    // Give the transport a moment to bind
    std::thread::sleep(std::time::Duration::from_millis(50));
    
    let msg = b"HELLO OMNI-MESH! THIS IS A LIVE TEST!";
    
    let header = EnvelopeHeader::new(
        1,
        MessageId::new([0xBB; 16]),
        Did::new([0x01; 32]), // sender
        Did::new([0x02; 32]), // recipient
        0,
        0,
        Priority::High,
        PayloadType::RobotCommand,
    );
    
    let mut payload = PayloadStorage::new();
    payload.push_bytes(msg).unwrap();
    
    let envelope = SignedEnvelope {
        header,
        payload,
        signature: [0u8; 64], // Unsigned for this demo, will be processed based on security mode
    };
    
    println!("Sending envelope to daemon at 127.0.0.1:9000...");
    match transport.send(&envelope) {
        Ok(_) => println!("Envelope sent successfully!"),
        Err(e) => eprintln!("Failed to send: {}", e),
    }
}
