import time
import omnimesh

def main():
    print("=== OMNI-MESH Python SDK Example ===\n")
    
    # Initialize the clients in development mode
    print("Starting Coordinator...")
    coordinator = omnimesh.Client("development")
    print(f"Coordinator DID: {coordinator.did}")
    
    print("Starting Robot...")
    robot = omnimesh.Client("development")
    print(f"Robot DID: {robot.did}\n")
    
    # Coordinator sends a task to the robot
    print("[Coordinator] Assigning task to Robot: 'pick' at 'A-12'")
    task_bytes = b"pick:A-12"
    # send_agent_command(target_did_hex, command_type, target_id, payload)
    coordinator.send_agent_command(robot.did, "pick", b"", task_bytes)
    
    time.sleep(0.1)
    
    # Robot receives the task
    print("\n=== Robot processing ===")
    msg = robot.receive(500)
    if msg:
        if msg.get("type") == "agent_command":
            cmd = msg.get("command_type")
            data = msg.get("payload").decode('utf-8')
            print(f"[Robot] ✓ Executing task: {cmd} with data {data}")
            
            # Robot sends heartbeat back
            print("[Robot] Sending telemetry heartbeat to Coordinator")
            robot.send_heartbeat(coordinator.did, 120000, 45, 1024, 1)
        else:
            print(f"[Robot] Received unexpected message type: {msg.get('type')}")
    else:
        print("[Robot] Timed out waiting for message")
        
    time.sleep(0.1)
    
    # Coordinator receives heartbeat
    print("\n=== Coordinator processing ===")
    msg = coordinator.receive(500)
    if msg:
        if msg.get("type") == "heartbeat":
            cpu = msg.get("cpu_usage")
            mem = msg.get("mem_usage_kb")
            print(f"[Coordinator] Received telemetry: CPU {cpu}%, Mem {mem}KB")
        else:
            print(f"[Coordinator] Received unexpected message type: {msg.get('type')}")
    else:
        print("[Coordinator] Timed out waiting for message")
        
    print("\n=== Python Example Complete ===")

if __name__ == "__main__":
    main()
