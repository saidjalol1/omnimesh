use omnimesh::buffer::PayloadStorage;
use omnimesh::envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};
use omnimesh::runtime::transport::config::TransportConfig;
use omnimesh::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use omnimesh::runtime::transport::tcp::TcpTransport;
use omnimesh::runtime::RoutingTable;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

fn query_ollama(prompt: &str) -> String {
    let client = reqwest::blocking::Client::new();
    let req = OllamaRequest {
        model: "tinyllama".to_string(),
        prompt: prompt.to_string(),
        stream: false,
    };

    match client.post("http://localhost:11434/api/generate")
        .json(&req)
        .send() 
    {
        Ok(resp) => {
            if let Ok(data) = resp.json::<OllamaResponse>() {
                data.response.trim().to_string()
            } else {
                "[Ollama parse error]".to_string()
            }
        }
        Err(_) => "[Ollama connection failed]".to_string(),
    }
}

fn create_chat_message(
    sender: Did,
    recipient: Did,
    msg_id: [u8; 16],
    msg: &str,
) -> SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY> {
    let header = EnvelopeHeader::new(
        1,
        MessageId::new(msg_id),
        sender,
        recipient,
        0,
        0,
        Priority::Normal,
        PayloadType::Raw,
    );

    let mut payload = PayloadStorage::new();
    payload.push_bytes(msg.as_bytes()).unwrap_or_default();

    SignedEnvelope {
        header,
        payload,
        signature: [0u8; 64],
    }
}

fn main() {
    println!("🤖 Initiating Real AI Agent Chat Environment over OMNI-MESH...");
    println!("Checking Ollama connection...");
    
    // Quick ping to Ollama
    let check = query_ollama("Say 'yes' if you are online.");
    if check.contains("failed") {
        eprintln!("Error: Could not connect to Ollama. Make sure you ran 'ollama run tinyllama' in another terminal.");
        return;
    }
    println!("✅ Ollama is online!");
    println!("============================================================");

    let alice_did = Did::new([0xAA; 32]);
    let bob_did = Did::new([0xBB; 32]);

    // Setup Bob
    let bob_config = TransportConfig::new(
        "127.0.0.1:9020".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let bob_routing = Arc::new(RoutingTable::new());
    bob_routing.update_route(alice_did, "127.0.0.1:9010".parse().unwrap());
    let bob_transport = TcpTransport::new(bob_config, bob_routing).unwrap();

    // Setup Alice
    let alice_config = TransportConfig::new(
        "127.0.0.1:9010".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let alice_routing = Arc::new(RoutingTable::new());
    alice_routing.update_route(bob_did, "127.0.0.1:9020".parse().unwrap());
    let alice_transport = TcpTransport::new(alice_config, alice_routing).unwrap();

    thread::sleep(Duration::from_millis(500));

    // Bob Thread
    let bob_thread = thread::spawn(move || {
        println!("🤖 [Agent Bob]: Online and listening...");
        
        loop {
            if let Some(envelope) = bob_transport.receive() {
                let text = std::str::from_utf8(envelope.payload.as_slice()).unwrap_or("<invalid utf8>");
                println!("\n📥 [Bob Received]: {}", text);
                
                println!("🧠 [Bob is thinking...]");
                let prompt = format!("You are an AI named Bob. You are having a casual conversation with your friend Alice. Alice just said: '{}'. Reply naturally to continue the conversation. Keep your response very short, maximum 2 sentences.", text);
                let ai_response = query_ollama(&prompt);
                
                // Truncate to fit OMNI-MESH 1024 byte limit safely using characters
                let safe_response = if ai_response.chars().count() > 250 {
                    let truncated: String = ai_response.chars().take(250).collect();
                    format!("{}...", truncated)
                } else {
                    ai_response.clone()
                };
                
                println!("📤 [Bob Sending]: {}", safe_response);
                
                let reply_env = create_chat_message(bob_did, alice_did, [0xBB; 16], &safe_response);
                bob_transport.send(&reply_env).unwrap();
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    // Alice Thread
    let alice_thread = thread::spawn(move || {
        println!("🤖 [Agent Alice]: Online. Initiating communication...");
        
        println!("🧠 [Alice is thinking...]");
        let initial_prompt = "You are an AI named Alice. You are about to start a casual conversation with your friend Bob. Say something interesting to start the chat. Keep it very short, maximum 2 sentences.";
        let mut msg_text = query_ollama(initial_prompt);
        
        // Truncate to fit safely using characters
        if msg_text.chars().count() > 250 {
            let truncated: String = msg_text.chars().take(250).collect();
            msg_text = format!("{}...", truncated);
        }
        
        println!("\n📤 [Alice Sending]: {}", msg_text);
        let mut envelope = create_chat_message(alice_did, bob_did, [0xAA; 16], &msg_text);
        alice_transport.send(&envelope).unwrap();

        // Alice enters infinite listening loop
        loop {
            if let Some(reply) = alice_transport.receive() {
                let text = std::str::from_utf8(reply.payload.as_slice()).unwrap_or("<invalid utf8>");
                println!("\n📥 [Alice Received]: {}", text);
                
                println!("🧠 [Alice is thinking...]");
                let prompt = format!("You are an AI named Alice. You are having a casual conversation with your friend Bob. Bob just said: '{}'. Reply naturally to continue the conversation. Keep your response very short, maximum 2 sentences.", text);
                let ai_response = query_ollama(&prompt);
                
                // Truncate
                let safe_response = if ai_response.len() > 900 {
                    format!("{}...", &ai_response[..900])
                } else {
                    ai_response.clone()
                };
                
                println!("📤 [Alice Sending]: {}", safe_response);
                
                let next_env = create_chat_message(alice_did, bob_did, [0xAA; 16], &safe_response);
                alice_transport.send(&next_env).unwrap();
            }
            thread::sleep(Duration::from_millis(100));
        }
    });

    // Infinite loop, let them chat until the user kills the process
    alice_thread.join().unwrap();
    bob_thread.join().unwrap();
}
