"""
OMNI-MESH Cross-Process Demo — Agent 1 (Sender)

Usage: Start BOTH agents within a few seconds of each other:
    Terminal 1:  python example_agent1.py
    Terminal 2:  python example_agent2.py
"""
import omnimesh
import json
import time
import os

DID_FILE = "dids.json"

# Create client on port 9001 (TCP mode)
client = omnimesh.Client(mode="lightweight", listen_port=9001)
print(f"[Agent1] Started on port 9001")
print(f"[Agent1] My DID: {client.did}")

# Save our DID immediately
dids = {}
if os.path.exists(DID_FILE):
    try:
        with open(DID_FILE, "r") as f:
            dids = json.load(f)
    except:
        dids = {}
dids["agent1"] = client.did
with open(DID_FILE, "w") as f:
    json.dump(dids, f)

# Wait for agent2's DID (up to 60 seconds)
print("[Agent1] Waiting for agent2...")
target_did = None
for i in range(60):
    try:
        with open(DID_FILE, "r") as f:
            dids = json.load(f)
        if "agent2" in dids:
            target_did = dids["agent2"]
            break
    except:
        pass
    time.sleep(1)
    if i % 10 == 9:
        print(f"[Agent1] Still waiting... ({i+1}s)")

if not target_did:
    print("[Agent1] Timeout: agent2 never came online.")
    client.shutdown()
    exit(1)

print(f"[Agent1] Found agent2: {target_did[:16]}...")
client.register_peer(target_did, "127.0.0.1:9002")
time.sleep(0.5)

# Send command
print("[Agent1] Sending 'pick' command...")
client.send_agent_command(target_did, "pick", b"robot-1", b"shelf-A12")

# Wait for response
print("[Agent1] Waiting for response...")
msg = client.receive(timeout_ms=15000)
if msg:
    print(f"[Agent1] GOT RESPONSE: {msg['command_type']} = {bytes(msg.get('payload', []))}")
else:
    print("[Agent1] No response (timeout)")

client.shutdown()
print("[Agent1] Done.")
