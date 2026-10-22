# omnimesh — Python SDK

Decentralized mesh networking for autonomous robot fleets and edge-AI systems.  
Native Rust performance. Pure Python API. Cryptographically signed messages.

```bash
pip install omnimesh
```

---

## Overview

omnimesh lets robots, AI nodes, and IoT devices communicate over a cryptographically secured mesh network. Each node has a unique identity (DID) derived from an Ed25519 keypair. Messages are signed automatically — tampering is detected and rejected.

```python
import omnimesh

# Each client = one node in the mesh
robot = omnimesh.Client(mode="lightweight", listen_port=9001)
print(robot.did)  # unique 64-char hex identity
```

---

## Modes

| Mode | Transport | Crypto | Use Case |
|------|-----------|--------|----------|
| `"development"` | In-process memory | Optional | Unit tests, CI, single-process demos |
| `"lightweight"` | TCP | Verify if signed | Multi-process, same machine or LAN |
| `"production"` | TCP + TLS | Required (rejects unsigned) | Fleet deployment |

```python
# Testing (no network, instant)
client = omnimesh.Client(mode="development")

# Real networking between processes
client = omnimesh.Client(mode="lightweight", listen_port=9001)

# Production with mandatory signature enforcement
client = omnimesh.Client(mode="production", listen_port=9001)
```

---

## Peer Discovery

Nodes find each other by registering peers manually. Each node needs to know:
1. The peer's DID (64-char hex string)
2. The peer's IP:port

```python
# On Robot A (port 9001)
robot_a = omnimesh.Client(mode="lightweight", listen_port=9001)

# On Robot B (port 9002)
robot_b = omnimesh.Client(mode="lightweight", listen_port=9002)

# Tell A where B is, and tell B where A is
robot_a.register_peer(robot_b.did, "192.168.1.42:9002")
robot_b.register_peer(robot_a.did, "192.168.1.41:9001")
```

For same-machine testing, use `127.0.0.1` with different ports.

---

## Sending Messages

### Agent Commands (general-purpose)

```python
client.send_agent_command(
    target_did_hex=peer_did,     # Who to send to
    command_type="pick",          # Command name
    target_id=b"robot-1",        # Target identifier
    payload_data=b"shelf-A12",   # Arbitrary bytes
)
```

### Motion Commands (ROS 2 geometry_msgs/Twist compatible)

```python
client.send_motion_command(
    target_did_hex=peer_did,
    linear_x=1.5,      # Forward velocity (m/s)
    linear_y=0.0,
    linear_z=0.0,
    angular_x=0.0,
    angular_y=0.0,
    angular_z=0.8,     # Turn rate (rad/s)
    deadline_ns=0,     # 0 = no deadline
)
```

### Heartbeats (telemetry)

```python
client.send_heartbeat(
    target_did_hex=peer_did,
    uptime_ms=60000,    # Node uptime
    cpu=75,             # CPU usage %
    mem_kb=4096,        # Memory usage
    epoch=1,            # Monotonic counter
)
```

### LLM Queries (edge AI inference)

```python
client.send_llm_query(
    target_did_hex=peer_did,
    prompt="What is the optimal path?",
    system_prompt="You are a navigation AI.",
    model="llama3",
)
```

---

## Receiving Messages

### Blocking (with timeout)

```python
# Waits up to 5 seconds, returns None if no message arrives
# GIL is released — other Python threads can run while waiting
msg = client.receive(timeout_ms=5000)
```

### Non-blocking

```python
msg = client.try_receive()  # Returns immediately
```

### Message format

All messages are Python dicts:

```python
{
    "type": "agent_command",       # Message type
    "sender_did": "a58a9b3b...",   # Who sent it
    "received_at_us": 1780382...,  # Receive timestamp (microseconds)
    "command_type": "pick",        # Type-specific fields...
    "target_did": "...",
    "payload": [115, 104, ...],    # Bytes as list of ints
}
```

Convert payload to bytes: `bytes(msg["payload"])`

### Message types and fields

| `msg["type"]` | Fields |
|---------------|--------|
| `"agent_command"` | `command_type`, `target_did`, `payload` |
| `"motion_command"` | `linear_x/y/z`, `angular_x/y/z`, `deadline_ns` |
| `"heartbeat"` | `uptime_ms`, `cpu_usage`, `mem_usage_kb`, `epoch` |
| `"llm_query"` | `prompt`, `system_prompt`, `model` |
| `"llm_response"` | `response`, `latency_us` |
| `"inference_result"` | `model_id`, `confidence`, `label`, `latency_us` |

---

## Lifecycle Management

```python
# Check queue depth
print(client.inbox_len())

# Graceful shutdown (stops background threads)
client.shutdown()

# Check state
client.is_shutdown()  # True after shutdown()

# Drain remaining messages after shutdown
while True:
    msg = client.try_receive()
    if msg is None:
        break
    process(msg)
```

---

## Production Deployment

### Two Robots on a LAN

**Robot A** (192.168.1.41):
```python
import omnimesh

robot_a = omnimesh.Client(mode="production", listen_port=9000)

# Register all known peers
robot_a.register_peer(ROBOT_B_DID, "192.168.1.42:9000")
robot_a.register_peer(ROBOT_C_DID, "192.168.1.43:9000")

# Send commands
robot_a.send_motion_command(
    target_did_hex=ROBOT_B_DID,
    linear_x=1.0, linear_y=0.0, linear_z=0.0,
    angular_x=0.0, angular_y=0.0, angular_z=0.0,
    deadline_ns=0,
)

# Receive
while True:
    msg = robot_a.receive(timeout_ms=1000)
    if msg:
        handle_message(msg)
```

**Robot B** (192.168.1.42):
```python
import omnimesh

robot_b = omnimesh.Client(mode="production", listen_port=9000)
robot_b.register_peer(ROBOT_A_DID, "192.168.1.41:9000")

while True:
    msg = robot_b.receive(timeout_ms=1000)
    if msg and msg["type"] == "motion_command":
        execute_velocity(msg["linear_x"], msg["angular_z"])
```

### Fleet Coordinator Pattern

```python
import omnimesh

coordinator = omnimesh.Client(mode="production", listen_port=9000)

# Register fleet
fleet = {
    "picker":     ("aabb...", "192.168.1.10:9000"),
    "transporter":("ccdd...", "192.168.1.11:9000"),
    "placer":     ("eeff...", "192.168.1.12:9000"),
}

for name, (did, addr) in fleet.items():
    coordinator.register_peer(did, addr)

# Assign tasks
for name, (did, _) in fleet.items():
    coordinator.send_agent_command(did, f"task-{name}", b"", b"zone-A")

# Collect heartbeats
while True:
    msg = coordinator.receive(timeout_ms=5000)
    if msg and msg["type"] == "heartbeat":
        print(f"Node {msg['sender_did'][:8]}: cpu={msg['cpu_usage']}%")
```

---

## Thread Safety

The client is fully thread-safe. `receive()` releases the Python GIL, so other threads can run concurrently.

```python
import omnimesh
import threading

client = omnimesh.Client(mode="lightweight", listen_port=9001)

def sender():
    while True:
        client.send_heartbeat(peer_did, uptime_ms=1000, cpu=50, mem_kb=2048, epoch=1)
        time.sleep(1)

def receiver():
    while True:
        msg = client.receive(timeout_ms=1000)
        if msg:
            process(msg)

threading.Thread(target=sender, daemon=True).start()
threading.Thread(target=receiver, daemon=True).start()
```

---

## Security Model

- Every node has an Ed25519 keypair generated at startup
- The DID is the public key (32 bytes, displayed as 64-char hex)
- Every message is signed with BLAKE3(header + payload) → Ed25519
- The receiver verifies the signature using the sender's DID as the public key
- In production mode, unsigned messages are rejected
- Tampered messages fail verification and are silently dropped

---

## Error Handling

```python
# Invalid DID format
try:
    client.send_agent_command("not-a-valid-did", "test", b"", b"")
except ValueError as e:
    print(e)  # "DID must be exactly 32 bytes (64 hex chars)"

# Sending after shutdown
try:
    client.shutdown()
    client.send_agent_command(peer_did, "test", b"", b"")
except ValueError as e:
    print(e)  # "Client has been shut down"

# Invalid mode
try:
    omnimesh.Client(mode="invalid")
except ValueError as e:
    print(e)  # "mode must be 'development', 'lightweight', or 'production'"
```

---

## API Reference

### `omnimesh.Client(mode="development", listen_port=None)`

Create a mesh node.

**Parameters:**
- `mode` — `"development"`, `"lightweight"`, or `"production"`
- `listen_port` — TCP port to listen on (required for lightweight/production)

**Properties:**
- `.did` → 64-char hex string (node identity)

**Methods:**

| Method | Description |
|--------|-------------|
| `register_peer(did_hex, addr)` | Register a peer's DID and IP:port |
| `send_agent_command(did, cmd, target_id, payload)` | Send a command |
| `send_motion_command(did, lx, ly, lz, ax, ay, az, deadline)` | Send velocity |
| `send_heartbeat(did, uptime_ms, cpu, mem_kb, epoch)` | Send telemetry |
| `send_llm_query(did, prompt, system_prompt, model)` | Send AI query |
| `receive(timeout_ms)` | Block until message or timeout |
| `try_receive()` | Non-blocking receive |
| `inbox_len()` | Messages in queue |
| `shutdown()` | Graceful stop |
| `is_shutdown()` | Check if stopped |

---

## Requirements

- Python 3.8+ (single `abi3` wheel covers 3.8–3.13)
- Platforms: **Linux x86_64**, **Linux ARM64**, **Windows x86_64**, **macOS x86_64**, **macOS Apple Silicon** — all via pre-built wheels
- For Android/embedded targets, use the Rust crate directly with `--no-default-features --features android`

## License

MIT
