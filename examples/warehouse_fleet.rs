//! Warehouse robot fleet simulation using the Developer SDK
//!
//! Simulates a fleet of warehouse robots coordinating tasks using OMNI-MESH.
//!
//! Scenario:
//! - 1 Coordinator node assigns tasks
//! - 3 Robot nodes execute tasks and report status back
//!
//! Usage:
//!   cargo run --example warehouse_fleet

use omnimesh::client::{ClientConfig, OmnimeshClient};
use omnimesh::payload;
use omnimesh::payload::PayloadKind;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== OMNI-MESH Warehouse Fleet Simulation (SDK Edition) ===\n");

    // Create coordinator
    let coordinator = OmnimeshClient::builder()
        .with_config(ClientConfig::development())
        .build()
        .expect("Failed to build Coordinator");

    println!(
        "Coordinator DID: {:?}\n",
        hex::encode(&coordinator.did.0[..8])
    );

    // Create robot fleet
    let mut robots = Vec::new();
    for i in 1..=3 {
        let robot = OmnimeshClient::builder()
            .with_config(ClientConfig::development())
            .build()
            .unwrap();
        println!("  Robot {} - DID: {:?}", i, hex::encode(&robot.did.0[..8]));
        robots.push((i, robot));
    }
    println!();

    // Coordinator assigns tasks
    println!("=== Task Assignment Phase ===\n");
    let tasks = vec![("pick", "A-12"), ("transport", "B-05"), ("place", "C-18")];

    for (i, (id, robot)) in robots.iter().enumerate() {
        let (task, location) = tasks[i];
        println!(
            "[Coordinator] Assigning task to Robot {}: {} at {}",
            id, task, location
        );

        let task_bytes = format!("{}:{}", task, location);
        let cmd = payload::agent_command(task, &robot.did.0, task_bytes.as_bytes());

        coordinator
            .send(robot.did, cmd)
            .expect("Failed to send task");
    }

    println!();
    thread::sleep(Duration::from_millis(100));

    // Robots process tasks and report back
    println!("=== Task Execution Phase ===\n");
    for (id, robot) in robots.iter() {
        if let Some(msg) = robot.receive_timeout(Duration::from_millis(500)) {
            if let Some(PayloadKind::AgentCommand(cmd)) = msg.payload.payload {
                let task_str = String::from_utf8_lossy(&cmd.payload);
                println!("[Robot {}] ✓ Executing task: {}", id, task_str);

                // Simulate task execution
                thread::sleep(Duration::from_millis(100));

                // Send heartbeat back to coordinator
                let heartbeat = payload::heartbeat(&robot.did.0, 60000, 45, 1024, 1);
                println!("[Robot {}] Sending telemetry heartbeat to Coordinator", id);
                robot
                    .send(coordinator.did, heartbeat)
                    .expect("Failed to send heartbeat");
            }
        }
    }

    println!();
    thread::sleep(Duration::from_millis(100));

    // Coordinator collects telemetry
    println!("=== Telemetry Collection Phase ===\n");
    let mut received_heartbeats = 0;
    while let Some(msg) = coordinator.try_receive() {
        if let Some(PayloadKind::Heartbeat(hb)) = msg.payload.payload {
            println!(
                "[Coordinator] Received heartbeat: CPU {}%, Mem {}KB",
                hb.cpu_usage, hb.mem_usage_kb
            );
            received_heartbeats += 1;
        }
    }

    println!(
        "\n[Coordinator] Total heartbeats received: {}",
        received_heartbeats
    );

    println!("\n=== Simulation Complete ===");
    println!("This example demonstrated:");
    println!("  ✓ Multi-node coordination via SDK");
    println!("  ✓ Task assignment and execution");
    println!("  ✓ Asynchronous non-blocking message routing");
    println!("  ✓ Strongly-typed payloads (AgentCommand, Heartbeat)");
}
