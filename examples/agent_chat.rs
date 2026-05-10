use omnimesh::buffer::PayloadStorage;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::transport::config::TransportConfig;
use omnimesh::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use omnimesh::runtime::transport::tcp::TcpTransport;
use omnimesh::runtime::RoutingTable;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Helper function to create an envelope containing a string message
fn create_chat_message(
    sender: Did,
    recipient: Did,
    msg_id: [u8; 16],
    msg: &str,
) -> SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY> {
    let header = EnvelopeHeader::new(
        1,
        MessageId::new(msg_id), // Passed message ID
        sender,
        recipient,
        0,
        0,
        Priority::Normal,
        PayloadType::Raw,
    );

    let mut payload = PayloadStorage::new();
    payload.push_bytes(msg.as_bytes()).unwrap();

    SignedEnvelope {
        header,
        payload,
        signature: [0u8; 64],
    }
}

fn main() {
    println!("🤖 Initiating Multi-Agent Chat Environment over OMNI-MESH...");
    println!("============================================================");

    let alice_did = Did::new([0xAA; 32]);
    let bob_did = Did::new([0xBB; 32]);

    // ---------------------------------------------------------
    // Setup AGENT BOB (Listens on 9020)
    // ---------------------------------------------------------
    let bob_config = TransportConfig::new(
        "127.0.0.1:9020".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let bob_routing = Arc::new(RoutingTable::new());
    // Bob needs to know how to route messages back to Alice
    bob_routing.update_route(alice_did, "127.0.0.1:9010".parse().unwrap());
    
    let bob_transport = TcpTransport::new(bob_config, bob_routing).unwrap();

    // ---------------------------------------------------------
    // Setup AGENT ALICE (Listens on 9010)
    // ---------------------------------------------------------
    let alice_config = TransportConfig::new(
        "127.0.0.1:9010".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let alice_routing = Arc::new(RoutingTable::new());
    // Alice needs to know how to route messages to Bob
    alice_routing.update_route(bob_did, "127.0.0.1:9020".parse().unwrap());
    
    let alice_transport = TcpTransport::new(alice_config, alice_routing).unwrap();

    // Give the sockets a moment to bind
    thread::sleep(Duration::from_millis(100));

    // =========================================================
    // BACKGROUND TASK: Agent Bob's Listening Loop
    // =========================================================
    let bob_thread = thread::spawn(move || {
        println!("🤖 [Agent Bob]: Online and listening...");
        
        // Wait for Alice's message
        loop {
            if let Some(envelope) = bob_transport.receive() {
                let text = std::str::from_utf8(envelope.payload.as_slice()).unwrap_or("<invalid utf8>");
                println!("\n📥 [Bob Received]: '{}'", text);
                
                thread::sleep(Duration::from_millis(800)); // Simulate AI thinking time

                // Send a reply back to Alice!
                let reply_text = "Status is nominal. All systems functioning correctly.";
                println!("📤 [Bob Sending]: '{}'", reply_text);
                
                let reply_env = create_chat_message(bob_did, alice_did, [0xBB; 16], reply_text);
                bob_transport.send(&reply_env).unwrap();
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    // =========================================================
    // BACKGROUND TASK: Agent Alice's Logic
    // =========================================================
    let alice_thread = thread::spawn(move || {
        println!("🤖 [Agent Alice]: Online. Initiating communication...");
        thread::sleep(Duration::from_millis(500));
        
        let msg_text = "Hello Bob, what is the status of the hyperdrive?";
        println!("\n📤 [Alice Sending]: '{}'", msg_text);
        
        let envelope = create_chat_message(alice_did, bob_did, [0xAA; 16], msg_text);
        alice_transport.send(&envelope).unwrap();

        // Wait for Bob's reply
        loop {
            if let Some(reply) = alice_transport.receive() {
                let text = std::str::from_utf8(reply.payload.as_slice()).unwrap_or("<invalid utf8>");
                println!("\n📥 [Alice Received]: '{}'", text);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
    });

    // Wait for the conversation to finish
    alice_thread.join().unwrap();
    bob_thread.join().unwrap();
    
    println!("\n✅ Chat Simulation Complete!");
}
