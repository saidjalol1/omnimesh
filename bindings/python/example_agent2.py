"""
OMNI-MESH Cross-Process Demo — Agent 2 (Receiver)

Usage: Start BOTH agents within a few seconds of each other:
    Terminal 1:  python example_agent1.py
    Terminal 2:  python example_agent2.py
"""
import omnimesh
import json
import time
import os

DID_FILE = "dids.json"

# Create client on port 9002 (TCP mode)
client = omnimesh.Client(mode="lightweight", listen_port=9002)
print(f"[Agent2] Started on port 9002")
print(f"[Agent2] My DID: {client.did}")

# Save our DID immediately
dids = {}
if os.path.exists(DID_FILE):
    try:
        with open(DID_FILE, "r") as f:
            dids = json.load(f)
    except:
        dids = {}
dids["agent2"] = client.did
with open(DID_FILE, "w") as f:
    json.dump(dids, f)

# Wait for agent1's DID (up to 60 seconds)
print("[Agent2] Waiting for agent1...")
target_did = None
for i in range(60):
    try:
        with open(DID_FILE, "r") as f:
            dids = json.load(f)
        if "agent1" in dids:
            target_did = dids["agent1"]
            break
    except:
        pass
    time.sleep(1)
    if i % 10 == 9:
        print(f"[Agent2] Still waiting... ({i+1}s)")

if not target_did:
    print("[Agent2] Timeout: agent1 never came online.")
    client.shutdown()
    exit(1)

print(f"[Agent2] Found agent1: {target_did[:16]}...")
client.register_peer(target_did, "127.0.0.1:9001")

# Wait for incoming message
print("[Agent2] Waiting for messages...")
msg = client.receive(timeout_ms=30000)
if msg:
    print(f"[Agent2] GOT: {msg['command_type']} = {bytes(msg.get('payload', []))}")

    # Send response back
    print("[Agent2] Sending 'ack' response...")
    client.send_agent_command(target_did, "ack", b"agent2", b"task-completed")
    time.sleep(1)
else:
    print("[Agent2] No message received (timeout)")

client.shutdown()
print("[Agent2] Done.")
