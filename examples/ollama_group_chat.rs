use omnimesh::client::{OmnimeshClient, ClientConfig};
use omnimesh::payload;
use omnimesh::payload::PayloadKind;
use serde::{Deserialize, Serialize};
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



fn main() {
    println!("🤖 Initiating 4-Agent Group Chat over OMNI-MESH (SDK Edition)...");
    
    let check = query_ollama("Say 'yes' if online.");
    if check.contains("failed") {
        eprintln!("Error: Ollama is offline. Please start ollama service locally.");
        return;
    }
    println!("✅ Ollama is online!");
    println!("============================================================");
    
    // We create the DIDs up front so we can form a ring, but our spawn_agent wrapper
    // generates the actual DIDs inside since the Client generates its own key.
    // Instead, let's just make the ring dynamically or let them talk to each other directly.
    // Actually, in the SDK we can't predetermine DIDs easily without passing Keys in.
    // Let's modify our ring approach to just spawn them and let them pass DIDs around,
    // OR we can just pass the target_did into a channel to start them!
    
    // For simplicity, let's just create 4 clients here in the main thread, 
    // connect their target DIDs, and move them into threads.
    
    let alice = OmnimeshClient::builder().with_config(ClientConfig::development()).build().unwrap();
    let bob = OmnimeshClient::builder().with_config(ClientConfig::development()).build().unwrap();
    let charlie = OmnimeshClient::builder().with_config(ClientConfig::development()).build().unwrap();
    let dave = OmnimeshClient::builder().with_config(ClientConfig::development()).build().unwrap();

    let alice_did = alice.did;
    let bob_did = bob.did;
    let charlie_did = charlie.did;
    let dave_did = dave.did;

    let run_agent = |name: &'static str, client: OmnimeshClient, target_did: omnimesh::envelope::Did, is_starter: bool| {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1500));

            if is_starter {
                println!("🧠 [{} is kicking off the group chat as the Holder...]", name);
                let prompt = format!("You are {}. Start a new conversation in a group chat with a short question (maximum 5 words) about a random topic. DO NOT write dialogue. DO NOT use your name. DO NOT use quotes.", name);
                let mut msg_text = query_ollama(&prompt);
                
                if let Some(idx) = msg_text.find('\n') { msg_text.truncate(idx); }
                msg_text = msg_text.replace("\"", "");
                
                let display_text = format!("{} says: {}", name, msg_text);
                println!("\n🗣️ [GROUP CHAT] {}", display_text);
                
                let msg = payload::agent_command("chat", b"", display_text.as_bytes());
                client.send(target_did, msg).unwrap();
            }

            loop {
                if let Some(envelope) = client.receive_timeout(Duration::from_millis(100)) {
                    if let Some(PayloadKind::AgentCommand(cmd)) = envelope.payload.payload {
                        let text = std::str::from_utf8(&cmd.payload).unwrap_or("<invalid utf8>");
                        
                        println!("🧠 [{} is thinking...]", name);
                        
                        let prompt = format!("You are {}. You are in a group chat. The last message was: '{}'. Reply with a very short statement (maximum 5 words). DO NOT write dialogue. DO NOT use your name. DO NOT use quotes.", name, text);
                        let mut ai_response = query_ollama(&prompt);
                        
                        if let Some(idx) = ai_response.find('\n') { ai_response.truncate(idx); }
                        ai_response = ai_response.replace("\"", "");
                        
                        let display_text = format!("{} says: {}", name, ai_response);
                        println!("\n🗣️ [GROUP CHAT] {}", display_text);
                        
                        let reply_msg = payload::agent_command("chat", b"", display_text.as_bytes());
                        client.send(target_did, reply_msg).unwrap();
                    }
                }
            }
        })
    };

    // Alice -> Bob -> Charlie -> Dave -> Alice
    let t_alice = run_agent("Alice", alice, bob_did, true);
    let _t_bob = run_agent("Bob", bob, charlie_did, false);
    let _t_charlie = run_agent("Charlie", charlie, dave_did, false);
    let _t_dave = run_agent("Dave", dave, alice_did, false);

    t_alice.join().unwrap();
    _t_bob.join().unwrap();
    _t_charlie.join().unwrap();
    _t_dave.join().unwrap();
}
