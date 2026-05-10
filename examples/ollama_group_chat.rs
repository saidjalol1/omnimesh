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
struct OllamaOptions {
    num_predict: usize,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

fn query_ollama(prompt: &str) -> String {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new());
        
    let req = OllamaRequest {
        model: "phi3".to_string(),
        prompt: prompt.to_string(),
        stream: false,
        options: OllamaOptions {
            num_predict: 20, // Force the model to stop generating after 20 tokens!
        },
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
        Err(e) => {
            eprintln!("⚠️ HTTP Error: {}", e);
            "[Ollama connection failed]".to_string()
        }
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

fn spawn_agent(
    name: &'static str,
    my_did: Did,
    my_port: u16,
    target_did: Did,
    target_port: u16,
    is_starter: bool,
) -> thread::JoinHandle<()> {
    
    // Setup transport
    let config = TransportConfig::new(
        format!("127.0.0.1:{}", my_port).parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    );
    let routing = Arc::new(RoutingTable::new());
    routing.update_route(target_did, format!("127.0.0.1:{}", target_port).parse().unwrap());
    
    let transport = TcpTransport::new(config, routing).unwrap();
    println!("🤖 [Agent {}]: Online on port {}...", name, my_port);

    thread::spawn(move || {
        // Wait for all agents to spin up
        thread::sleep(Duration::from_millis(1500));

        if is_starter {
            println!("🧠 [{} is kicking off the group chat as the Holder...]", name);
            let prompt = format!("You are {}. Start a new conversation in a group chat with a short question (maximum 5 words) about a random topic. DO NOT write dialogue. DO NOT use your name. DO NOT use quotes.", name);
            let mut msg_text = query_ollama(&prompt);
            
            // Clean up LLM hallucinations
            if let Some(idx) = msg_text.find('\n') {
                msg_text.truncate(idx); // Take only the first line
            }
            if let Some(stripped) = msg_text.strip_prefix(&format!("{}:", name)) {
                msg_text = stripped.trim().to_string();
            } else if let Some(stripped) = msg_text.strip_prefix(&format!("{} says:", name)) {
                msg_text = stripped.trim().to_string();
            }
            msg_text = msg_text.replace("\"", "");
            
            if msg_text.chars().count() > 800 {
                let truncated: String = msg_text.chars().take(800).collect();
                msg_text = format!("{}...", truncated);
            }
            
            let display_text = format!("{} says: {}", name, msg_text);
            println!("\n🗣️ [GROUP CHAT] {}", display_text);
            
            let envelope = create_chat_message(my_did, target_did, [0xAA; 16], &display_text);
            transport.send(&envelope).unwrap();
        }

        loop {
            if let Some(envelope) = transport.receive() {
                let text = std::str::from_utf8(envelope.payload.as_slice()).unwrap_or("<invalid utf8>");
                
                println!("🧠 [{} is thinking...]", name);
                
                let prompt = format!("You are {}. You are in a group chat. The last message was: '{}'. Reply with a very short statement (maximum 5 words). DO NOT write dialogue. DO NOT use your name. DO NOT use quotes.", name, text);
                let mut ai_response = query_ollama(&prompt);
                
                // Clean up LLM hallucinations
                if let Some(idx) = ai_response.find('\n') {
                    ai_response.truncate(idx); // Take only the first line
                }
                if let Some(stripped) = ai_response.strip_prefix(&format!("{}:", name)) {
                    ai_response = stripped.trim().to_string();
                } else if let Some(stripped) = ai_response.strip_prefix(&format!("{} says:", name)) {
                    ai_response = stripped.trim().to_string();
                }
                ai_response = ai_response.replace("\"", "");
                
                if ai_response.chars().count() > 800 {
                    let truncated: String = ai_response.chars().take(800).collect();
                    ai_response = format!("{}...", truncated);
                }
                
                let display_text = format!("{} says: {}", name, ai_response);
                println!("\n🗣️ [GROUP CHAT] {}", display_text);
                
                let reply_env = create_chat_message(my_did, target_did, [0xAA; 16], &display_text);
                transport.send(&reply_env).unwrap();
            }
            thread::sleep(Duration::from_millis(100));
        }
    })
}

fn main() {
    println!("🤖 Initiating 4-Agent Group Chat over OMNI-MESH...");
    
    let check = query_ollama("Say 'yes' if online.");
    if check.contains("failed") {
        eprintln!("Error: Ollama is offline.");
        return;
    }
    println!("✅ Ollama is online!");
    println!("============================================================");
    
    // Agent DIDs
    let alice_did = Did::new([0xA1; 32]);
    let bob_did = Did::new([0xB2; 32]);
    let charlie_did = Did::new([0xC3; 32]);
    let dave_did = Did::new([0xD4; 32]);

    // Create a Token Ring: Alice -> Bob -> Charlie -> Dave -> Alice
    let t_dave = spawn_agent("Dave", dave_did, 9040, alice_did, 9010, false);
    let t_charlie = spawn_agent("Charlie", charlie_did, 9030, dave_did, 9040, false);
    let t_bob = spawn_agent("Bob", bob_did, 9020, charlie_did, 9030, false);
    let t_alice = spawn_agent("Alice", alice_did, 9010, bob_did, 9020, true);

    t_alice.join().unwrap();
    t_bob.join().unwrap();
    t_charlie.join().unwrap();
    t_dave.join().unwrap();
}
